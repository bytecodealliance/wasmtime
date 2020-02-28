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
use wasmtime_environ::wasm::{DefinedFuncIndex, DefinedMemoryIndex, MemoryIndex};
use wasmtime_environ::{
    CacheConfig, Compilation, CompileError, CompiledFunction, CompiledFunctionUnwindInfo,
    Compiler as _C, FunctionBodyData, Module, ModuleMemoryOffset, ModuleVmctxInfo, Relocations,
    Traps, Tunables, VMOffsets,
};
use wasmtime_profiling::ProfilingAgent;
use wasmtime_runtime::{
    InstantiationError, SignatureRegistry, TrapRegistration, TrapRegistry, VMFunctionBody,
    VMSharedSignatureIndex,
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
    trap_registry: TrapRegistry,
    trampoline_park: HashMap<VMSharedSignatureIndex, *const VMFunctionBody>,
    signatures: SignatureRegistry,
    strategy: CompilationStrategy,
    cache_config: CacheConfig,

    /// The `FunctionBuilderContext`, shared between trampline function compilations.
    fn_builder_ctx: FunctionBuilderContext,
}

impl Compiler {
    /// Construct a new `Compiler`.
    pub fn new(
        isa: Box<dyn TargetIsa>,
        strategy: CompilationStrategy,
        cache_config: CacheConfig,
    ) -> Self {
        Self {
            isa,
            code_memory: CodeMemory::new(),
            trampoline_park: HashMap::new(),
            signatures: SignatureRegistry::new(),
            fn_builder_ctx: FunctionBuilderContext::new(),
            strategy,
            trap_registry: TrapRegistry::default(),
            cache_config,
        }
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
            TrapRegistration,
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
                        &self.cache_config,
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
                        &self.cache_config,
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

        let trap_registration = register_traps(&allocated_functions, &traps, &self.trap_registry);

        // Translate debug info (DWARF) only if at least one function is present.
        let dbg = if debug_data.is_some() && !allocated_functions.is_empty() {
            let target_config = self.isa.frontend_config();
            let ofs = VMOffsets::new(target_config.pointer_bytes(), &module.local);

            let mut funcs = Vec::new();
            for (i, allocated) in allocated_functions.into_iter() {
                let ptr = (*allocated) as *const u8;
                let body_len = compilation.get(i).body.len();
                funcs.push((ptr, body_len));
            }
            let module_vmctx_info = {
                ModuleVmctxInfo {
                    memory_offset: if ofs.num_imported_memories > 0 {
                        ModuleMemoryOffset::Imported(ofs.vmctx_vmmemory_import(MemoryIndex::new(0)))
                    } else if ofs.num_defined_memories > 0 {
                        ModuleMemoryOffset::Defined(
                            ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)),
                        )
                    } else {
                        ModuleMemoryOffset::None
                    },
                    stack_slots,
                }
            };
            let bytes = emit_debugsections_image(
                self.isa.triple().clone(),
                target_config,
                debug_data.as_ref().unwrap(),
                &module_vmctx_info,
                &address_transform,
                &value_ranges,
                &funcs,
            )
            .map_err(SetupError::DebugInfo)?;
            Some(bytes)
        } else {
            None
        };

        let jt_offsets = compilation.get_jt_offsets();

        Ok((
            allocated_functions,
            jt_offsets,
            relocations,
            dbg,
            trap_registration,
        ))
    }

    /// Create a trampoline for invoking a function.
    pub(crate) fn get_trampoline(
        &mut self,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, SetupError> {
        let index = self.signatures.register(signature);
        if let Some(trampoline) = self.trampoline_park.get(&index) {
            return Ok(*trampoline);
        }
        let body = make_trampoline(
            &*self.isa,
            &mut self.code_memory,
            &mut self.fn_builder_ctx,
            signature,
            value_size,
        )?;
        self.trampoline_park.insert(index, body);
        return Ok(body);
    }

    /// Create and publish a trampoline for invoking a function.
    pub fn get_published_trampoline(
        &mut self,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, SetupError> {
        let result = self.get_trampoline(signature, value_size)?;
        self.publish_compiled_code();
        Ok(result)
    }

    /// Make memory containing compiled code executable.
    pub(crate) fn publish_compiled_code(&mut self) {
        self.code_memory.publish();
    }

    pub(crate) fn profiler_module_load(
        &mut self,
        profiler: &mut Box<dyn ProfilingAgent + Send>,
        module_name: &str,
        dbg_image: Option<&[u8]>,
    ) -> () {
        self.code_memory
            .profiler_module_load(profiler, module_name, dbg_image);
    }

    /// Shared signature registry.
    pub fn signatures(&self) -> &SignatureRegistry {
        &self.signatures
    }

    /// Shared registration of trap information
    pub fn trap_registry(&self) -> &TrapRegistry {
        &self.trap_registry
    }
}

/// Create a trampoline for invoking a function.
fn make_trampoline(
    isa: &dyn TargetIsa,
    code_memory: &mut CodeMemory,
    fn_builder_ctx: &mut FunctionBuilderContext,
    signature: &ir::Signature,
    value_size: usize,
) -> Result<*const VMFunctionBody, SetupError> {
    let pointer_type = isa.pointer_type();
    let mut wrapper_sig = ir::Signature::new(isa.frontend_config().default_call_conv);

    // Add the callee `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));

    // Add the caller `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    // Add the `callee_address` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    // Add the `values_vec` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    let mut context = Context::new();
    context.func = ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wrapper_sig);
    context.func.collect_frame_layout_info();

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let (vmctx_ptr_val, caller_vmctx_ptr_val, callee_value, values_vec_ptr_val) = {
            let params = builder.func.dfg.block_params(block0);
            (params[0], params[1], params[2], params[3])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        let callee_args = signature
            .params
            .iter()
            .enumerate()
            .map(|(i, r)| {
                match i {
                    0 => vmctx_ptr_val,
                    1 => caller_vmctx_ptr_val,
                    _ =>
                    // i - 2 because vmctx and caller vmctx aren't passed through `values_vec`.
                    {
                        builder.ins().load(
                            r.value_type,
                            mflags,
                            values_vec_ptr_val,
                            ((i - 2) * value_size) as i32,
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        let new_sig = builder.import_signature(signature.clone());

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

    let unwind_info = CompiledFunctionUnwindInfo::new(isa, &context);

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
    registry: &TrapRegistry,
) -> TrapRegistration {
    let traps =
        allocated_functions
            .values()
            .zip(traps.values())
            .flat_map(|(func_addr, func_traps)| {
                func_traps.iter().map(move |trap_desc| {
                    let func_addr = *func_addr as *const u8 as usize;
                    let offset = usize::try_from(trap_desc.code_offset).unwrap();
                    let trap_addr = func_addr + offset;
                    (trap_addr, trap_desc.source_loc, trap_desc.trap_code)
                })
            });
    registry.register_traps(traps)
}

/// We don't expect trampoline compilation to produce any relocations, so
/// this `RelocSink` just asserts that it doesn't recieve any.
struct RelocSink {}

impl binemit::RelocSink for RelocSink {
    fn reloc_block(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _block_offset: binemit::CodeOffset,
    ) {
        panic!("trampoline compilation should not produce block relocs");
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
