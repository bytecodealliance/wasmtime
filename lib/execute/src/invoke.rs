//! Support for invoking wasm functions from outside a wasm module.

use code::Code;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::{binemit, ir, isa, Context};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_wasm::FuncIndex;
use signalhandlers::{ensure_eager_signal_handlers, ensure_full_signal_handlers, TrapContext};
use std::mem;
use std::ptr;
use std::string::String;
use std::vec::Vec;
use traphandlers::call_wasm;
use vmcontext::VMContext;
use wasmtime_environ::{Compilation, Export, Module, RelocSink};

/// A runtime value.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// A runtime value with type i32.
    I32(i32),
    /// A runtime value with type i64.
    I64(i64),
    /// A runtime value with type f32.
    F32(u32),
    /// A runtime value with type f64.
    F64(u64),
}

impl Value {
    /// Return the type of this `Value`.
    pub fn value_type(self) -> ir::Type {
        match self {
            Value::I32(_) => ir::types::I32,
            Value::I64(_) => ir::types::I64,
            Value::F32(_) => ir::types::F32,
            Value::F64(_) => ir::types::F64,
        }
    }

    /// Assuming this `Value` holds an `i32`, return that value.
    pub fn unwrap_i32(self) -> i32 {
        match self {
            Value::I32(x) => x,
            _ => panic!("unwrapping value of type {} as i32", self.value_type()),
        }
    }

    /// Assuming this `Value` holds an `i64`, return that value.
    pub fn unwrap_i64(self) -> i64 {
        match self {
            Value::I64(x) => x,
            _ => panic!("unwrapping value of type {} as i64", self.value_type()),
        }
    }

    /// Assuming this `Value` holds an `f32`, return that value.
    pub fn unwrap_f32(self) -> u32 {
        match self {
            Value::F32(x) => x,
            _ => panic!("unwrapping value of type {} as f32", self.value_type()),
        }
    }

    /// Assuming this `Value` holds an `f64`, return that value.
    pub fn unwrap_f64(self) -> u64 {
        match self {
            Value::F64(x) => x,
            _ => panic!("unwrapping value of type {} as f64", self.value_type()),
        }
    }
}

/// The result of invoking a wasm function.
#[derive(Debug)]
pub enum InvokeOutcome {
    /// The function returned normally. Its return values are provided.
    Returned {
        /// The return values.
        values: Vec<Value>,
    },
    /// A trap occurred while the function was executing.
    Trapped {
        /// The trap message.
        message: String,
    },
}

/// Jumps to the code region of memory and invoke the exported function
pub fn invoke(
    code: &mut Code,
    isa: &isa::TargetIsa,
    module: &Module,
    compilation: &Compilation,
    vmctx: *mut VMContext,
    function: &str,
    args: &[Value],
) -> Result<InvokeOutcome, String> {
    let fn_index = match module.exports.get(function) {
        Some(Export::Function(index)) => *index,
        Some(_) => return Err(format!("exported item \"{}\" is not a function", function)),
        None => return Err(format!("no export named \"{}\"", function)),
    };

    invoke_by_index(code, isa, module, compilation, vmctx, fn_index, args)
}

pub fn invoke_by_index(
    code: &mut Code,
    isa: &isa::TargetIsa,
    module: &Module,
    compilation: &Compilation,
    vmctx: *mut VMContext,
    fn_index: FuncIndex,
    args: &[Value],
) -> Result<InvokeOutcome, String> {
    let code_buf = &compilation.functions[module
        .defined_func_index(fn_index)
        .expect("imported start functions not supported yet")];
    let sig = &module.signatures[module.functions[fn_index]];

    let exec_code_buf = code.allocate_copy_of_slice(&code_buf)?.as_ptr();

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
        return Err("failed to install signal handlers".to_string());
    }

    call_through_wrapper(code, isa, exec_code_buf as usize, vmctx, args, &sig)
}

fn call_through_wrapper(
    code: &mut Code,
    isa: &isa::TargetIsa,
    callee: usize,
    vmctx: *mut VMContext,
    args: &[Value],
    sig: &ir::Signature,
) -> Result<InvokeOutcome, String> {
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
                Value::I32(i) => {
                    callee_args.push(builder.ins().iconst(ir::types::I32, i64::from(*i)))
                }
                Value::I64(i) => callee_args.push(builder.ins().iconst(ir::types::I64, *i)),
                Value::F32(i) => callee_args.push(
                    builder
                        .ins()
                        .f32const(ir::immediates::Ieee32::with_bits(*i)),
                ),
                Value::F64(i) => callee_args.push(
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
        .map_err(|e| e.to_string())?;
    assert!(reloc_sink.func_relocs.is_empty());

    let exec_code_buf = code.allocate_copy_of_slice(&code_buf)?.as_ptr();
    code.publish();

    let func = unsafe { mem::transmute::<_, fn()>(exec_code_buf) };

    Ok(match call_wasm(func) {
        Ok(()) => {
            let mut values = Vec::with_capacity(sig.returns.len());

            for (index, abi_param) in sig.returns.iter().enumerate() {
                let v = unsafe {
                    let ptr = results_vec.as_ptr().add(index * value_size);

                    match abi_param.value_type {
                        ir::types::I32 => Value::I32(ptr::read(ptr as *const i32)),
                        ir::types::I64 => Value::I64(ptr::read(ptr as *const i64)),
                        ir::types::F32 => Value::F32(ptr::read(ptr as *const u32)),
                        ir::types::F64 => Value::F64(ptr::read(ptr as *const u64)),
                        other => panic!("unsupported value type {:?}", other),
                    }
                };

                values.push(v);
            }

            InvokeOutcome::Returned { values }
        }
        Err(message) => InvokeOutcome::Trapped { message },
    })
}
