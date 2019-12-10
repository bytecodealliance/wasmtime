//! JIT compilation.

use crate::code_memory::CodeMemory;
use crate::instantiate::SetupError;
use crate::target_tunables::target_tunables;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_wasm::ModuleTranslationState;
use std::collections::HashMap;
use std::convert::TryFrom;
use wasmtime_debug::{emit_debugsections_image, DebugInfoData};
use wasmtime_environ::entity::{EntityRef, PrimaryMap};
use wasmtime_environ::isa::{TargetFrontendConfig, TargetIsa};
use wasmtime_environ::wasm::{DefinedFuncIndex, DefinedMemoryIndex};
use wasmtime_environ::{
    Compilation, CompileError, CompiledFunction, Compiler as _C, FunctionBodyData, Module,
    ModuleVmctxInfo, Relocations, Traps, Tunables, VMOffsets,
};
use wasmtime_runtime::{
    get_mut_trap_registry, InstantiationError, SignatureRegistry, TrapRegistrationGuard,
    VMFunctionBody,
};

/// Select which kind of compilation to use.
#[derive(Copy, Clone, Debug)]
pub enum CompilationStrategy {
    /// Let Wasmtime pick the strategy.
    Auto,

    /// Compile all functions with Cranelift.
    Cranelift,

    /// Compile all functions with Lightbeam.
    #[cfg(feature = "lightbeam")]
    Lightbeam,
}

/// A WebAssembly code JIT compiler.
///
/// A `Compiler` instance owns the executable memory that it allocates.
///
/// TODO: Evolve this to support streaming rather than requiring a `&[u8]`
/// containing a whole wasm module at once.
///
/// TODO: Consider using cranelift-module.
pub struct Compiler {
    isa: Box<dyn TargetIsa>,

    code_memory: CodeMemory,
    trap_registration_guards: Vec<TrapRegistrationGuard>,
    trampoline_park: HashMap<*const VMFunctionBody, *const VMFunctionBody>,
    signatures: SignatureRegistry,
    strategy: CompilationStrategy,

    /// The `FunctionBuilderContext`, shared between trampline function compilations.
    fn_builder_ctx: FunctionBuilderContext,
}

impl Compiler {
    /// Construct a new `Compiler`.
    pub fn new(isa: Box<dyn TargetIsa>, strategy: CompilationStrategy) -> Self {
        Self {
            isa,
            code_memory: CodeMemory::new(),
            trap_registration_guards: Vec::new(),
            trampoline_park: HashMap::new(),
            signatures: SignatureRegistry::new(),
            fn_builder_ctx: FunctionBuilderContext::new(),
            strategy,
        }
    }
}

impl Drop for Compiler {
    fn drop(&mut self) {
        // We must deregister traps before freeing the code memory.
        // Otherwise, we have a race:
        // - Compiler #1 dropped code memory, but hasn't deregistered the trap yet
        // - Compiler #2 allocated code memory and tries to register a trap,
        //   but the trap at certain address happens to be already registered,
        //   since Compiler #1 hasn't deregistered it yet => assertion in trap registry fails.
        // Having a custom drop implementation we are independent from the field order
        // in the struct what reduces potential human error.
        self.trap_registration_guards.clear();
    }
}

impl Compiler {
    /// Return the target's frontend configuration settings.
    pub fn frontend_config(&self) -> TargetFrontendConfig {
        self.isa.frontend_config()
    }

    /// Return the tunables in use by this engine.
    pub fn tunables(&self) -> Tunables {
        target_tunables(self.isa.triple())
    }

    /// Compile the given function bodies.
    pub(crate) fn compile<'data>(
        &mut self,
        module: &Module,
        module_translation: &ModuleTranslationState,
        function_body_inputs: PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        debug_data: Option<DebugInfoData>,
    ) -> Result<
        (
            PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
            PrimaryMap<DefinedFuncIndex, ir::JumpTableOffsets>,
            Relocations,
            Option<Vec<u8>>,
        ),
        SetupError,
    > {
        let (compilation, relocations, address_transform, value_ranges, stack_slots, traps) =
            match self.strategy {
                // For now, interpret `Auto` as `Cranelift` since that's the most stable
                // implementation.
                CompilationStrategy::Auto | CompilationStrategy::Cranelift => {
                    wasmtime_environ::cranelift::Cranelift::compile_module(
                        module,
                        module_translation,
                        function_body_inputs,
                        &*self.isa,
                        debug_data.is_some(),
                    )
                }
                #[cfg(feature = "lightbeam")]
                CompilationStrategy::Lightbeam => {
                    wasmtime_environ::lightbeam::Lightbeam::compile_module(
                        module,
                        module_translation,
                        function_body_inputs,
                        &*self.isa,
                        debug_data.is_some(),
                    )
                }
            }
            .map_err(SetupError::Compile)?;

        let allocated_functions =
            allocate_functions(&mut self.code_memory, &compilation).map_err(|message| {
                SetupError::Instantiate(InstantiationError::Resource(format!(
                    "failed to allocate memory for functions: {}",
                    message
                )))
            })?;

        register_traps(
            &allocated_functions,
            &traps,
            &mut self.trap_registration_guards,
        );

        let dbg = if let Some(debug_data) = debug_data {
            let target_config = self.isa.frontend_config();
            let ofs = VMOffsets::new(target_config.pointer_bytes(), &module);
            if ofs.num_defined_memories > 0 {
                let mut funcs = Vec::new();
                for (i, allocated) in allocated_functions.into_iter() {
                    let ptr = (*allocated) as *const u8;
                    let body_len = compilation.get(i).body.len();
                    funcs.push((ptr, body_len));
                }
                let module_vmctx_info = {
                    let memory_offset =
                        ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)) as i64;
                    ModuleVmctxInfo {
                        memory_offset,
                        stack_slots,
                    }
                };
                let bytes = emit_debugsections_image(
                    self.isa.triple().clone(),
                    target_config,
                    &debug_data,
                    &module_vmctx_info,
                    &address_transform,
                    &value_ranges,
                    &funcs,
                )
                .map_err(SetupError::DebugInfo)?;
                Some(bytes)
            } else {
                None
            }
        } else {
            None
        };

        let jt_offsets = compilation.get_jt_offsets();

        Ok((allocated_functions, jt_offsets, relocations, dbg))
    }

    /// Create a trampoline for invoking a function.
    pub(crate) fn get_trampoline(
        &mut self,
        callee_address: *const VMFunctionBody,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, SetupError> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        Ok(match self.trampoline_park.entry(callee_address) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => {
                let body = make_trampoline(
                    &*self.isa,
                    &mut self.code_memory,
                    &mut self.fn_builder_ctx,
                    callee_address,
                    signature,
                    value_size,
                )?;
                entry.insert(body);
                body
            }
        })
    }

    /// Create and publish a trampoline for invoking a function.
    pub fn get_published_trampoline(
        &mut self,
        callee_address: *const VMFunctionBody,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, SetupError> {
        let result = self.get_trampoline(callee_address, signature, value_size)?;
        self.publish_compiled_code();
        Ok(result)
    }

    /// Make memory containing compiled code executable.
    pub(crate) fn publish_compiled_code(&mut self) {
        self.code_memory.publish();
    }

    /// Shared signature registry.
    pub fn signatures(&mut self) -> &mut SignatureRegistry {
        &mut self.signatures
    }
}

/// Create a trampoline for invoking a function.
fn make_trampoline(
    isa: &dyn TargetIsa,
    code_memory: &mut CodeMemory,
    fn_builder_ctx: &mut FunctionBuilderContext,
    callee_address: *const VMFunctionBody,
    signature: &ir::Signature,
    value_size: usize,
) -> Result<*const VMFunctionBody, SetupError> {
    let pointer_type = isa.pointer_type();
    let mut wrapper_sig = ir::Signature::new(isa.frontend_config().default_call_conv);

    // Add the `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));
    // Add the `values_vec` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    let mut context = Context::new();
    context.func = ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wrapper_sig);

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_ebb();

        builder.append_ebb_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let (vmctx_ptr_val, values_vec_ptr_val) = {
            let params = builder.func.dfg.ebb_params(block0);
            (params[0], params[1])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        let callee_args = signature
            .params
            .iter()
            .enumerate()
            .map(|(i, r)| {
                match r.purpose {
                    // i - 1 because vmctx isn't passed through `values_vec`.
                    ir::ArgumentPurpose::Normal => builder.ins().load(
                        r.value_type,
                        mflags,
                        values_vec_ptr_val,
                        ((i - 1) * value_size) as i32,
                    ),
                    ir::ArgumentPurpose::VMContext => vmctx_ptr_val,
                    other => panic!("unsupported argument purpose {}", other),
                }
            })
            .collect::<Vec<_>>();

        let new_sig = builder.import_signature(signature.clone());

        // TODO: It's possible to make this a direct call. We just need Cranelift
        // to support functions declared with an immediate integer address.
        // ExternalName::Absolute(u64). Let's do it.
        let callee_value = builder.ins().iconst(pointer_type, callee_address as i64);
        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let results = builder.func.dfg.inst_results(call).to_vec();

        // Store the return values into `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, values_vec_ptr_val, (i * value_size) as i32);
        }

        builder.ins().return_(&[]);
        builder.finalize()
    }

    let mut code_buf = Vec::new();
    let mut unwind_info = Vec::new();
    let mut reloc_sink = RelocSink {};
    let mut trap_sink = binemit::NullTrapSink {};
    let mut stackmap_sink = binemit::NullStackmapSink {};
    context
        .compile_and_emit(
            isa,
            &mut code_buf,
            &mut reloc_sink,
            &mut trap_sink,
            &mut stackmap_sink,
        )
        .map_err(|error| {
            SetupError::Compile(CompileError::Codegen(pretty_error(
                &context.func,
                Some(isa),
                error,
            )))
        })?;

    context.emit_unwind_info(isa, &mut unwind_info);

    Ok(code_memory
        .allocate_for_function(&CompiledFunction {
            body: code_buf,
            jt_offsets: context.func.jt_offsets,
            unwind_info,
        })
        .map_err(|message| SetupError::Instantiate(InstantiationError::Resource(message)))?
        .as_ptr())
}

fn allocate_functions(
    code_memory: &mut CodeMemory,
    compilation: &Compilation,
) -> Result<PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>, String> {
    let fat_ptrs = code_memory.allocate_for_compilation(compilation)?;

    // Second, create a PrimaryMap from result vector of pointers.
    let mut result = PrimaryMap::with_capacity(compilation.len());
    for i in 0..fat_ptrs.len() {
        let fat_ptr: *mut [VMFunctionBody] = fat_ptrs[i];
        result.push(fat_ptr);
    }
    Ok(result)
}

fn register_traps(
    allocated_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    traps: &Traps,
    trap_registration_guards: &mut Vec<TrapRegistrationGuard>,
) {
    let mut trap_registry = get_mut_trap_registry();
    for (func_addr, func_traps) in allocated_functions.values().zip(traps.values()) {
        for trap_desc in func_traps.iter() {
            let func_addr = *func_addr as *const u8 as usize;
            let offset = usize::try_from(trap_desc.code_offset).unwrap();
            let trap_addr = func_addr + offset;
            let guard =
                trap_registry.register_trap(trap_addr, trap_desc.source_loc, trap_desc.trap_code);
            trap_registration_guards.push(guard);
        }
    }
}

/// We don't expect trampoline compilation to produce any relocations, so
/// this `RelocSink` just asserts that it doesn't recieve any.
struct RelocSink {}

impl binemit::RelocSink for RelocSink {
    fn reloc_ebb(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _ebb_offset: binemit::CodeOffset,
    ) {
        panic!("trampoline compilation should not produce ebb relocs");
    }
    fn reloc_external(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _name: &ir::ExternalName,
        _addend: binemit::Addend,
    ) {
        panic!("trampoline compilation should not produce external symbol relocs");
    }
    fn reloc_constant(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _constant_offset: ir::ConstantOffset,
    ) {
        panic!("trampoline compilation should not produce constant relocs");
    }
    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        panic!("trampoline compilation should not produce jump table relocs");
    }
}
