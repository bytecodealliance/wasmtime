#![no_main]
use libfuzzer_sys::fuzz_target;

use crate::codegen::ir::Function;
use arbitrary::Unstructured;
use cranelift::codegen::data_value::DataValue;
use cranelift::prelude::*;
use cranelift_filetests::function_runner::SingleFunctionCompiler;
use cranelift_fuzzgen::*;
use cranelift_interpreter::environment::FuncIndex;
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{Interpreter, InterpreterState};
use cranelift_interpreter::step::ControlFlow;
use cranelift_interpreter::step::CraneliftTrap;

enum RunResult {
    Success(Vec<DataValue>),
    Trap(CraneliftTrap),
    Error(Box<dyn std::error::Error>),
}

fn run_in_interpreter(func: &Function, args: &[DataValue]) -> RunResult {
    let mut env = FunctionStore::default();
    env.add(func.name.to_string(), func);

    let state = InterpreterState::default().with_function_store(env);
    let mut interpreter = Interpreter::new(state);

    // The entrypoint function is always 0
    let index = FuncIndex::from_u32(0);
    let res = interpreter.call_by_index(index, args);
    match res {
        Ok(ControlFlow::Return(results)) => RunResult::Success(results.to_vec()),
        Ok(ControlFlow::Trap(trap)) => RunResult::Trap(trap),
        Ok(cf) => RunResult::Error(format!("Unrecognized exit ControlFlow: {:?}", cf).into()),
        Err(e) => RunResult::Error(format!("InterpreterError: {:?}", e).into()),
    }
}

fn run_in_host(func: &Function, args: &[DataValue]) -> RunResult {
    let mut compiler = SingleFunctionCompiler::with_default_host_isa();

    match compiler.compile(func.clone()) {
        Ok(compiled_fn) => {
            // TODO: What happens if we trap here?
            let res = compiled_fn.call(args);
            RunResult::Success(res)
        }
        Err(e) => RunResult::Error(Box::new(e)),
    }
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    let mut fuzzgen = FuzzGen::new(&mut u);
    let testcase = match fuzzgen.generate_test() {
        Ok(test) => test,

        // arbitrary Errors mean that the fuzzer didn't give us enough input data
        Err(e) if e.is::<arbitrary::Error>() => {
            return;
        }
        Err(e) => std::panic::panic_any(e),
    };

    let func = testcase.func;
    let args = &testcase.inputs[0];

    let int_res = run_in_interpreter(&func, &args[..]);
    if let RunResult::Error(e) = int_res {
        panic!("interpreter failed: {}", e);
    }

    let host_res = run_in_host(&func, &args[..]);
    if let RunResult::Error(e) = host_res {
        panic!("host failed: {}", e);
    }

    // match (int_res, host_res) {
    //
    // }
});
