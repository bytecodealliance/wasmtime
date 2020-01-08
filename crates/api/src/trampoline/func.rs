//! Support for a calling of an imported function.

use super::create_handle::create_handle;
use super::trap::{record_api_trap, TrapSink, API_TRAP_CODE};
use crate::{Callable, FuncType, Store, Val};
use anyhow::Result;
use std::cmp;
use std::convert::TryFrom;
use std::rc::Rc;
use wasmtime_environ::entity::{EntityRef, PrimaryMap};
use wasmtime_environ::ir::types;
use wasmtime_environ::isa::TargetIsa;
use wasmtime_environ::wasm::{DefinedFuncIndex, FuncIndex};
use wasmtime_environ::{ir, settings, CompiledFunction, Export, Module, TrapInformation};
use wasmtime_jit::trampoline::ir::{
    ExternalName, Function, InstBuilder, MemFlags, StackSlotData, StackSlotKind,
};
use wasmtime_jit::trampoline::{
    binemit, pretty_error, Context, FunctionBuilder, FunctionBuilderContext,
};
use wasmtime_jit::{native, CodeMemory};
use wasmtime_runtime::{
    get_mut_trap_registry, InstanceHandle, TrapRegistrationGuard, VMContext, VMFunctionBody,
};

struct TrampolineState {
    func: Rc<dyn Callable + 'static>,
    #[allow(dead_code)]
    code_memory: CodeMemory,
    trap_registration_guards: Vec<TrapRegistrationGuard>,
}

impl TrampolineState {
    fn new(
        func: Rc<dyn Callable + 'static>,
        code_memory: CodeMemory,
        func_addr: *const VMFunctionBody,
        func_traps: &[TrapInformation],
    ) -> Self {
        let mut trap_registry = get_mut_trap_registry();
        let mut trap_registration_guards = Vec::new();
        for trap_desc in func_traps.iter() {
            let func_addr = func_addr as *const u8 as usize;
            let offset = usize::try_from(trap_desc.code_offset).unwrap();
            let trap_addr = func_addr + offset;
            let guard =
                trap_registry.register_trap(trap_addr, trap_desc.source_loc, trap_desc.trap_code);
            trap_registration_guards.push(guard);
        }
        TrampolineState {
            func,
            code_memory,
            trap_registration_guards,
        }
    }
}

impl Drop for TrampolineState {
    fn drop(&mut self) {
        // We must deregister traps before freeing the code memory.
        self.trap_registration_guards.clear();
    }
}

unsafe extern "C" fn stub_fn(vmctx: *mut VMContext, call_id: u32, values_vec: *mut i128) -> u32 {
    let mut instance = InstanceHandle::from_vmctx(vmctx);

    let (args, returns_len) = {
        let module = instance.module_ref();
        let signature = &module.signatures[module.functions[FuncIndex::new(call_id as usize)]];

        let mut args = Vec::new();
        for i in 1..signature.params.len() {
            args.push(Val::read_value_from(
                values_vec.offset(i as isize - 1),
                signature.params[i].value_type,
            ))
        }
        (args, signature.returns.len())
    };

    let mut returns = vec![Val::null(); returns_len];
    let func = &instance
        .host_state()
        .downcast_mut::<TrampolineState>()
        .expect("state")
        .func;

    match func.call(&args, &mut returns) {
        Ok(()) => {
            for (i, r#return) in returns.iter_mut().enumerate() {
                // TODO check signature.returns[i].value_type ?
                r#return.write_value_to(values_vec.add(i));
            }
            0
        }
        Err(trap) => {
            record_api_trap(trap);
            1
        }
    }
}

/// Create a trampoline for invoking a Callable.
fn make_trampoline(
    isa: &dyn TargetIsa,
    code_memory: &mut CodeMemory,
    fn_builder_ctx: &mut FunctionBuilderContext,
    call_id: u32,
    signature: &ir::Signature,
) -> (*const VMFunctionBody, Vec<TrapInformation>) {
    // Mostly reverse copy of the similar method from wasmtime's
    // wasmtime-jit/src/compiler.rs.
    let pointer_type = isa.pointer_type();
    let mut stub_sig = ir::Signature::new(isa.frontend_config().default_call_conv);

    // Add the `vmctx` parameter.
    stub_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));

    // Add the `call_id` parameter.
    stub_sig.params.push(ir::AbiParam::new(types::I32));

    // Add the `values_vec` parameter.
    stub_sig.params.push(ir::AbiParam::new(pointer_type));

    // Add error/trap return.
    stub_sig.returns.push(ir::AbiParam::new(types::I32));

    let value_size = 16;
    let values_vec_len = ((value_size as usize)
        * cmp::max(signature.params.len() - 1, signature.returns.len()))
        as u32;

    let mut context = Context::new();
    context.func = Function::with_name_signature(ExternalName::user(0, 0), signature.clone());

    let ss = context.func.create_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        values_vec_len,
    ));

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_ebb();

        builder.append_ebb_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let values_vec_ptr_val = builder.ins().stack_addr(pointer_type, ss, 0);
        let mflags = MemFlags::trusted();
        for i in 1..signature.params.len() {
            if i == 0 {
                continue;
            }

            let val = builder.func.dfg.ebb_params(block0)[i];
            builder.ins().store(
                mflags,
                val,
                values_vec_ptr_val,
                ((i - 1) * value_size) as i32,
            );
        }

        let vmctx_ptr_val = builder.func.dfg.ebb_params(block0)[0];
        let call_id_val = builder.ins().iconst(types::I32, call_id as i64);

        let callee_args = vec![vmctx_ptr_val, call_id_val, values_vec_ptr_val];

        let new_sig = builder.import_signature(stub_sig);

        let callee_value = builder
            .ins()
            .iconst(pointer_type, stub_fn as *const VMFunctionBody as i64);
        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let call_result = builder.func.dfg.inst_results(call)[0];
        builder.ins().trapnz(call_result, API_TRAP_CODE);

        let mflags = MemFlags::trusted();
        let mut results = Vec::new();
        for (i, r) in signature.returns.iter().enumerate() {
            let load = builder.ins().load(
                r.value_type,
                mflags,
                values_vec_ptr_val,
                (i * value_size) as i32,
            );
            results.push(load);
        }
        builder.ins().return_(&results);
        builder.finalize()
    }

    let mut code_buf: Vec<u8> = Vec::new();
    let mut reloc_sink = binemit::TrampolineRelocSink {};
    let mut trap_sink = TrapSink::new();
    let mut stackmap_sink = binemit::NullStackmapSink {};
    context
        .compile_and_emit(
            isa,
            &mut code_buf,
            &mut reloc_sink,
            &mut trap_sink,
            &mut stackmap_sink,
        )
        .map_err(|error| pretty_error(&context.func, Some(isa), error))
        .expect("compile_and_emit");

    let mut unwind_info = Vec::new();
    context.emit_unwind_info(isa, &mut unwind_info);

    let traps = trap_sink.traps;

    let addr = code_memory
        .allocate_for_function(&CompiledFunction {
            body: code_buf,
            jt_offsets: context.func.jt_offsets,
            unwind_info,
        })
        .expect("allocate_for_function")
        .as_ptr();
    (addr, traps)
}

pub fn create_handle_with_function(
    ft: &FuncType,
    func: &Rc<dyn Callable + 'static>,
    store: &Store,
) -> Result<InstanceHandle> {
    let sig = ft.get_wasmtime_signature().clone();

    let isa = {
        let isa_builder = native::builder();
        let flag_builder = settings::builder();
        isa_builder.finish(settings::Flags::new(flag_builder))
    };

    let mut fn_builder_ctx = FunctionBuilderContext::new();
    let mut module = Module::new();
    let mut finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody> =
        PrimaryMap::new();
    let mut code_memory = CodeMemory::new();

    let sig_id = module.signatures.push(sig.clone());
    let func_id = module.functions.push(sig_id);
    module
        .exports
        .insert("trampoline".to_string(), Export::Function(func_id));
    let (trampoline, traps) = make_trampoline(
        isa.as_ref(),
        &mut code_memory,
        &mut fn_builder_ctx,
        func_id.index() as u32,
        &sig,
    );
    code_memory.publish();

    finished_functions.push(trampoline);

    let trampoline_state = TrampolineState::new(func.clone(), code_memory, trampoline, &traps);

    create_handle(
        module,
        Some(store),
        finished_functions,
        Box::new(trampoline_state),
    )
}
