#![no_main]

use libfuzzer_sys::fuzz_target;

use arbitrary::Unstructured;
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::Function;
use cranelift::prelude::*;
use cranelift_filetests::function_runner::{CompiledFunction, SingleFunctionCompiler};
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

fn run_in_interpreter(interpreter: &mut Interpreter, args: &[DataValue]) -> RunResult {
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

fn run_in_host(compiled_fn: &CompiledFunction, args: &[DataValue]) -> RunResult {
    // TODO: What happens if we trap here?
    let res = compiled_fn.call(args);
    RunResult::Success(res)
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

    let mut interpreter = {
        let mut env = FunctionStore::default();
        env.add(testcase.func.name.to_string(), &testcase.func);

        let state = InterpreterState::default().with_function_store(env);
        let interpreter = Interpreter::new(state);
        interpreter
    };

    // Native fn
    let mut host_compiler = SingleFunctionCompiler::with_default_host_isa();
    let compiled_fn = host_compiler.compile(testcase.func.clone()).unwrap();

    for args in &testcase.inputs {
        let int_res = run_in_interpreter(&mut interpreter, args);
        if let RunResult::Error(e) = int_res {
            panic!("interpreter failed: {}", e);
        }

        let host_res = run_in_host(&compiled_fn, args);
        if let RunResult::Error(e) = host_res {
            panic!("host failed: {}", e);
        }

        match (int_res, host_res) {
            (RunResult::Success(lhs), RunResult::Success(rhs)) if lhs == rhs => {
                return;
            }
            (RunResult::Trap(lhs), RunResult::Trap(rhs)) if lhs == rhs => {
                return;
            }
            _ => panic!("Host and Interpreter disagree"),
        }
    }
});
