use action::ActionError;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir, isa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use jit_code::JITCode;
use std::collections::HashMap;
use std::fmt;
use wasmtime_environ::{CompileError, RelocSink};
use wasmtime_runtime::{InstantiationError, VMFunctionBody};

pub struct TrampolinePark {
    /// Memoized per-function trampolines.
    memoized: HashMap<*const VMFunctionBody, *const VMFunctionBody>,

    /// The `FunctionBuilderContext`, shared between function compilations.
    fn_builder_ctx: FunctionBuilderContext,
}

impl TrampolinePark {
    pub fn new() -> Self {
        Self {
            memoized: HashMap::new(),
            fn_builder_ctx: FunctionBuilderContext::new(),
        }
    }

    pub fn get(
        &mut self,
        jit_code: &mut JITCode,
        isa: &isa::TargetIsa,
        callee_address: *const VMFunctionBody,
        signature: &ir::Signature,
        value_size: usize,
    ) -> Result<*const VMFunctionBody, ActionError> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        Ok(match self.memoized.entry(callee_address) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => {
                let body = make_trampoline(
                    &mut self.fn_builder_ctx,
                    jit_code,
                    isa,
                    callee_address,
                    signature,
                    value_size,
                )?;
                entry.insert(body);
                body
            }
        })
    }
}

impl fmt::Debug for TrampolinePark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // The `fn_builder_ctx` field is just a cache and has no logical state.
        write!(f, "{:?}", self.memoized)
    }
}

fn make_trampoline(
    fn_builder_ctx: &mut FunctionBuilderContext,
    jit_code: &mut JITCode,
    isa: &isa::TargetIsa,
    callee_address: *const VMFunctionBody,
    signature: &ir::Signature,
    value_size: usize,
) -> Result<*const VMFunctionBody, ActionError> {
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
        let pointer_type = isa.pointer_type();

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
    let mut reloc_sink = RelocSink::new();
    let mut trap_sink = binemit::NullTrapSink {};
    context
        .compile_and_emit(isa, &mut code_buf, &mut reloc_sink, &mut trap_sink)
        .map_err(|error| ActionError::Compile(CompileError::Codegen(error)))?;
    assert!(reloc_sink.func_relocs.is_empty());

    Ok(jit_code
        .allocate_copy_of_byte_slice(&code_buf)
        .map_err(|message| ActionError::Instantiate(InstantiationError::Resource(message)))?
        .as_ptr())
}
