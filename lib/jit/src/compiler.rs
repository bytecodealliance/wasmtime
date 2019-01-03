//! JIT compilation.

use super::HashMap;
use crate::code_memory::CodeMemory;
use crate::instantiate::SetupError;
use crate::target_tunables::target_tunables;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::isa::{TargetFrontendConfig, TargetIsa};
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir};
use cranelift_entity::PrimaryMap;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_wasm::DefinedFuncIndex;
use std::boxed::Box;
use std::string::String;
use std::vec::Vec;
use wasmtime_environ::cranelift;
use wasmtime_environ::{Compilation, CompileError, Module, Relocations, Tunables};
use wasmtime_runtime::{InstantiationError, SignatureRegistry, VMFunctionBody};

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
    trampoline_park: HashMap<*const VMFunctionBody, *const VMFunctionBody>,
    signatures: SignatureRegistry,

    /// The `FunctionBuilderContext`, shared between trampline function compilations.
    fn_builder_ctx: FunctionBuilderContext,
}

impl Compiler {
    /// Construct a new `Compiler`.
    pub fn new(isa: Box<dyn TargetIsa>) -> Self {
        Self {
            isa,
            code_memory: CodeMemory::new(),
            trampoline_park: HashMap::new(),
            signatures: SignatureRegistry::new(),
            fn_builder_ctx: FunctionBuilderContext::new(),
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
        function_body_inputs: PrimaryMap<DefinedFuncIndex, &'data [u8]>,
    ) -> Result<
        (
            PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
            Relocations,
        ),
        SetupError,
    > {
        let (compilation, relocations) =
            cranelift::compile_module(module, function_body_inputs, &*self.isa)
                .map_err(SetupError::Compile)?;

        let allocated_functions =
            allocate_functions(&mut self.code_memory, compilation).map_err(|message| {
                SetupError::Instantiate(InstantiationError::Resource(format!(
                    "failed to allocate memory for functions: {}",
                    message
                )))
            })?;

        Ok((allocated_functions, relocations))
    }

    /// Create a trampoline for invoking a function.
    pub(crate) fn get_trampoline(
        &mut self,
        callee_address: *const VMFunctionBody,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, SetupError> {
        use super::hash_map::Entry::{Occupied, Vacant};
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

    /// Make memory containing compiled code executable.
    pub(crate) fn publish_compiled_code(&mut self) {
        self.code_memory.publish();
    }

    pub(crate) fn signatures(&mut self) -> &mut SignatureRegistry {
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

    // Add the `values_vec` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));
    // Add the `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));

    let mut context = Context::new();
    context.func = ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wrapper_sig);

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_ebb();

        builder.append_ebb_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let mut callee_args = Vec::new();
        let (values_vec_ptr_val, vmctx_ptr_val) = {
            let params = builder.func.dfg.ebb_params(block0);
            (params[0], params[1])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in signature.params.iter().enumerate() {
            let value = match r.purpose {
                ir::ArgumentPurpose::Normal => builder.ins().load(
                    r.value_type,
                    mflags,
                    values_vec_ptr_val,
                    (i * value_size) as i32,
                ),
                ir::ArgumentPurpose::VMContext => vmctx_ptr_val,
                other => panic!("unsupported argument purpose {}", other),
            };
            callee_args.push(value);
        }

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

    let mut code_buf: Vec<u8> = Vec::new();
    let mut reloc_sink = RelocSink {};
    let mut trap_sink = binemit::NullTrapSink {};
    context
        .compile_and_emit(isa, &mut code_buf, &mut reloc_sink, &mut trap_sink)
        .map_err(|error| SetupError::Compile(CompileError::Codegen(error)))?;

    Ok(code_memory
        .allocate_copy_of_byte_slice(&code_buf)
        .map_err(|message| SetupError::Instantiate(InstantiationError::Resource(message)))?
        .as_ptr())
}

fn allocate_functions(
    code_memory: &mut CodeMemory,
    compilation: Compilation,
) -> Result<PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>, String> {
    let mut result = PrimaryMap::with_capacity(compilation.functions.len());
    for (_, body) in compilation.functions.into_iter() {
        let fatptr: *mut [VMFunctionBody] = code_memory.allocate_copy_of_byte_slice(body)?;
        result.push(fatptr);
    }
    Ok(result)
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
    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        panic!("trampoline compilation should not produce jump table relocs");
    }
}
