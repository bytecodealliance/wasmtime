#![allow(missing_docs)]

use crate::code_memory::CodeMemory;
use crate::instantiate::SetupError;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::isa::TargetIsa;
use wasmtime_environ::{CompileError, CompiledFunction, Relocation, RelocationTarget};
use wasmtime_runtime::{InstantiationError, VMFunctionBody, VMTrampoline};

pub mod ir {
    pub(super) use cranelift_codegen::ir::{
        AbiParam, ArgumentPurpose, ConstantOffset, JumpTable, Signature, SourceLoc,
    };
    pub use cranelift_codegen::ir::{
        ExternalName, Function, InstBuilder, MemFlags, StackSlotData, StackSlotKind,
    };
}
pub use cranelift_codegen::print_errors::pretty_error;
pub use cranelift_codegen::Context;
pub use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

pub mod binemit {
    pub use cranelift_codegen::binemit::NullTrapSink;
    pub(super) use cranelift_codegen::binemit::{Addend, Reloc, RelocSink};
    pub use cranelift_codegen::binemit::{CodeOffset, NullStackmapSink, TrapSink};
}

/// Create a trampoline for invoking a function.
pub fn make_trampoline(
    isa: &dyn TargetIsa,
    code_memory: &mut CodeMemory,
    fn_builder_ctx: &mut FunctionBuilderContext,
    signature: &ir::Signature,
    value_size: usize,
) -> Result<VMTrampoline, SetupError> {
    let (compiled_function, relocs) = build_trampoline(isa, fn_builder_ctx, signature, value_size)?;

    assert!(relocs.is_empty());
    let ptr = code_memory
        .allocate_for_function(&compiled_function)
        .map_err(|message| SetupError::Instantiate(InstantiationError::Resource(message)))?
        .as_ptr();
    Ok(unsafe { std::mem::transmute::<*const VMFunctionBody, VMTrampoline>(ptr) })
}

pub(crate) fn build_trampoline(
    isa: &dyn TargetIsa,
    fn_builder_ctx: &mut FunctionBuilderContext,
    signature: &ir::Signature,
    value_size: usize,
) -> Result<(CompiledFunction, Vec<Relocation>), SetupError> {
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
    let mut reloc_sink = TrampolineRelocSink::default();
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

    let unwind_info = context.create_unwind_info(isa).map_err(|error| {
        SetupError::Compile(CompileError::Codegen(pretty_error(
            &context.func,
            Some(isa),
            error,
        )))
    })?;

    Ok((
        CompiledFunction {
            body: code_buf,
            jt_offsets: context.func.jt_offsets,
            unwind_info,
        },
        reloc_sink.relocs,
    ))
}

/// We don't expect trampoline compilation to produce many relocations, so
/// this `RelocSink` just asserts that it doesn't recieve most of them, but
/// handles libcall ones.
#[derive(Default)]
pub struct TrampolineRelocSink {
    relocs: Vec<Relocation>,
}

impl TrampolineRelocSink {
    /// Returns collected relocations.
    pub fn relocs(&self) -> &[Relocation] {
        &self.relocs
    }
}

impl binemit::RelocSink for TrampolineRelocSink {
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
        offset: binemit::CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: binemit::Reloc,
        name: &ir::ExternalName,
        addend: binemit::Addend,
    ) {
        let reloc_target = if let ir::ExternalName::LibCall(libcall) = *name {
            RelocationTarget::LibCall(libcall)
        } else {
            panic!("unrecognized external name")
        };
        self.relocs.push(Relocation {
            reloc,
            reloc_target,
            offset,
            addend,
        });
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
