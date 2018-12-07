//! Support for invoking wasm functions from outside a wasm module.

use action::{ActionError, ActionOutcome, RuntimeValue};
use code::Code;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::{binemit, ir, isa, Context};
use cranelift_entity::EntityRef;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_wasm::FuncIndex;
use instance::Instance;
use signalhandlers::{ensure_eager_signal_handlers, ensure_full_signal_handlers, TrapContext};
use std::mem;
use std::ptr;
use std::vec::Vec;
use traphandlers::call_wasm;
use vmcontext::VMContext;
use wasmtime_environ::{CompileError, Export, Module, RelocSink};

/// Calls the given named function, passing its return values and returning
/// its results.
pub fn invoke(
    code: &mut Code,
    isa: &isa::TargetIsa,
    module: &Module,
    instance: &mut Instance,
    function: &str,
    args: &[RuntimeValue],
) -> Result<ActionOutcome, ActionError> {
    let fn_index = match module.exports.get(function) {
        Some(Export::Function(index)) => *index,
        Some(_) => {
            return Err(ActionError::Kind(format!(
                "exported item \"{}\" is not a function",
                function
            )))
        }
        None => {
            return Err(ActionError::Field(format!(
                "no export named \"{}\"",
                function
            )))
        }
    };

    invoke_by_index(code, isa, module, instance, fn_index, args)
}

/// Invoke the WebAssembly start function of the instance, if one is present.
pub fn invoke_start_function(
    code: &mut Code,
    isa: &isa::TargetIsa,
    module: &Module,
    instance: &mut Instance,
) -> Result<ActionOutcome, ActionError> {
    if let Some(start_index) = module.start_func {
        invoke_by_index(code, isa, module, instance, start_index, &[])
    } else {
        // No start function, just return nothing.
        Ok(ActionOutcome::Returned { values: vec![] })
    }
}

/// Calls the given indexed function, passing its return values and returning
/// its results.
pub fn invoke_by_index(
    code: &mut Code,
    isa: &isa::TargetIsa,
    module: &Module,
    instance: &mut Instance,
    fn_index: FuncIndex,
    args: &[RuntimeValue],
) -> Result<ActionOutcome, ActionError> {
    let exec_code_buf = match module.defined_func_index(fn_index) {
        Some(def_fn_index) => {
            let slice = instance
                .get_allocated_function(def_fn_index)
                .ok_or_else(|| ActionError::Index(def_fn_index.index() as u64))?;
            code.allocate_copy_of_slice(slice)
                .map_err(ActionError::Resource)?
                .as_ptr()
        }
        None => instance
            .get_imported_function(fn_index)
            .ok_or_else(|| ActionError::Index(fn_index.index() as u64))?,
    };

    let sig = &module.signatures[module.functions[fn_index]];

    // TODO: Move this out to be done once per thread rather than per call.
    let mut traps = TrapContext {
        triedToInstallSignalHandlers: false,
        haveSignalHandlers: false,
    };

    // Rather than writing inline assembly to jump to the code region, we use the fact that
    // the Rust ABI for calling a function with no arguments and no return values matches the one
    // of the generated code. Thanks to this, we can transmute the code region into a first-class
    // Rust function and call it.
    // Ensure that our signal handlers are ready for action.
    ensure_eager_signal_handlers();
    ensure_full_signal_handlers(&mut traps);
    if !traps.haveSignalHandlers {
        return Err(ActionError::Resource(
            "failed to install signal handlers".to_string(),
        ));
    }

    call_through_wrapper(code, isa, exec_code_buf, instance, args, &sig)
}

fn call_through_wrapper(
    code: &mut Code,
    isa: &isa::TargetIsa,
    callee: *const u8,
    instance: &mut Instance,
    args: &[RuntimeValue],
    sig: &ir::Signature,
) -> Result<ActionOutcome, ActionError> {
    let vmctx = instance.vmctx() as *mut VMContext;

    for (index, value) in args.iter().enumerate() {
        assert_eq!(value.value_type(), sig.params[index].value_type);
    }

    let wrapper_sig = ir::Signature::new(isa.frontend_config().default_call_conv);
    let mut context = Context::new();
    context.func = ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wrapper_sig);

    let value_size = 8;
    let mut results_vec = Vec::new();
    results_vec.resize(sig.returns.len(), 0i64);

    let mut fn_builder_ctx = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut fn_builder_ctx);
        let block0 = builder.create_ebb();

        builder.append_ebb_params_for_function_params(block0);

        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let mut callee_args = Vec::new();
        let pointer_type = isa.pointer_type();

        let callee_value = builder.ins().iconst(pointer_type, callee as i64);

        for value in args {
            match value {
                RuntimeValue::I32(i) => {
                    callee_args.push(builder.ins().iconst(ir::types::I32, i64::from(*i)))
                }
                RuntimeValue::I64(i) => callee_args.push(builder.ins().iconst(ir::types::I64, *i)),
                RuntimeValue::F32(i) => callee_args.push(
                    builder
                        .ins()
                        .f32const(ir::immediates::Ieee32::with_bits(*i)),
                ),
                RuntimeValue::F64(i) => callee_args.push(
                    builder
                        .ins()
                        .f64const(ir::immediates::Ieee64::with_bits(*i)),
                ),
            }
        }

        let vmctx_value = builder.ins().iconst(pointer_type, vmctx as i64);
        callee_args.push(vmctx_value);

        let new_sig = builder.import_signature(sig.clone());

        // TODO: It's possible to make this a direct call. We just need Cranelift
        // to support functions declared with an immediate integer address.
        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let results = builder.func.dfg.inst_results(call).to_vec();

        let results_vec_value = builder
            .ins()
            .iconst(pointer_type, results_vec.as_ptr() as i64);

        let mut mflags = ir::MemFlags::new();
        mflags.set_notrap();
        mflags.set_aligned();
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, results_vec_value, (i * value_size) as i32);
        }

        builder.ins().return_(&[]);
    }

    let mut code_buf: Vec<u8> = Vec::new();
    let mut reloc_sink = RelocSink::new();
    let mut trap_sink = binemit::NullTrapSink {};
    context
        .compile_and_emit(isa, &mut code_buf, &mut reloc_sink, &mut trap_sink)
        .map_err(|error| ActionError::Compile(CompileError::Codegen(error)))?;
    assert!(reloc_sink.func_relocs.is_empty());

    let exec_code_buf = code
        .allocate_copy_of_slice(&code_buf)
        .map_err(ActionError::Resource)?
        .as_ptr();
    code.publish();

    let func = unsafe { mem::transmute::<_, fn()>(exec_code_buf) };

    Ok(match call_wasm(func) {
        Ok(()) => {
            let mut values = Vec::with_capacity(sig.returns.len());

            for (index, abi_param) in sig.returns.iter().enumerate() {
                let v = unsafe {
                    let ptr = results_vec.as_ptr().add(index * value_size);

                    match abi_param.value_type {
                        ir::types::I32 => RuntimeValue::I32(ptr::read(ptr as *const i32)),
                        ir::types::I64 => RuntimeValue::I64(ptr::read(ptr as *const i64)),
                        ir::types::F32 => RuntimeValue::F32(ptr::read(ptr as *const u32)),
                        ir::types::F64 => RuntimeValue::F64(ptr::read(ptr as *const u64)),
                        other => panic!("unsupported value type {:?}", other),
                    }
                };

                values.push(v);
            }

            ActionOutcome::Returned { values }
        }
        Err(message) => ActionOutcome::Trapped { message },
    })
}
