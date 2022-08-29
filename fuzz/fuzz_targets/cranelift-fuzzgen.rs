#![no_main]

use libfuzzer_sys::fuzz_target;

use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::LibCall;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_filetests::function_runner::{TestFileCompiler, Trampoline};
use cranelift_fuzzgen::*;
use cranelift_interpreter::environment::FuncIndex;
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{
    Interpreter, InterpreterError, InterpreterState, LibCallValues,
};
use cranelift_interpreter::step::ControlFlow;
use cranelift_interpreter::step::CraneliftTrap;
use smallvec::{smallvec, SmallVec};

const INTERPRETER_FUEL: u64 = 4096;

#[derive(Debug)]
enum RunResult {
    Success(Vec<DataValue>),
    Trap(CraneliftTrap),
    Timeout,
    Error(Box<dyn std::error::Error>),
}

impl RunResult {
    pub fn unwrap(self) -> Vec<DataValue> {
        match self {
            RunResult::Success(d) => d,
            _ => panic!("Expected RunResult::Success in unwrap but got: {:?}", self),
        }
    }
}

fn run_in_interpreter(interpreter: &mut Interpreter, args: &[DataValue]) -> RunResult {
    // The entrypoint function is always 0
    let index = FuncIndex::from_u32(0);
    let res = interpreter.call_by_index(index, args);

    match res {
        Ok(ControlFlow::Return(results)) => RunResult::Success(results.to_vec()),
        Ok(ControlFlow::Trap(trap)) => RunResult::Trap(trap),
        Ok(cf) => RunResult::Error(format!("Unrecognized exit ControlFlow: {:?}", cf).into()),
        Err(InterpreterError::FuelExhausted) => RunResult::Timeout,
        Err(e) => RunResult::Error(e.into()),
    }
}

fn run_in_host(trampoline: &Trampoline, args: &[DataValue]) -> RunResult {
    let res = trampoline.call(args);
    RunResult::Success(res)
}

fn build_interpreter(testcase: &TestCase) -> Interpreter {
    let mut env = FunctionStore::default();
    env.add(testcase.func.name.to_string(), &testcase.func);

    let state = InterpreterState::default()
        .with_function_store(env)
        .with_libcall_handler(|libcall: LibCall, args: LibCallValues<DataValue>| {
            use LibCall::*;
            Ok(smallvec![match (libcall, &args[..]) {
                (CeilF32, [DataValue::F32(a)]) => DataValue::F32(a.ceil()),
                (CeilF64, [DataValue::F64(a)]) => DataValue::F64(a.ceil()),
                (FloorF32, [DataValue::F32(a)]) => DataValue::F32(a.floor()),
                (FloorF64, [DataValue::F64(a)]) => DataValue::F64(a.floor()),
                (TruncF32, [DataValue::F32(a)]) => DataValue::F32(a.trunc()),
                (TruncF64, [DataValue::F64(a)]) => DataValue::F64(a.trunc()),
                _ => unreachable!(),
            }])
        });

    let interpreter = Interpreter::new(state).with_fuel(Some(INTERPRETER_FUEL));
    interpreter
}

fuzz_target!(|testcase: TestCase| {
    // Native fn
    let flags = {
        let mut builder = settings::builder();
        // We need llvm ABI extensions for i128 values on x86
        builder.set("enable_llvm_abi_extensions", "true").unwrap();
        settings::Flags::new(builder)
    };
    let mut compiler = TestFileCompiler::with_host_isa(flags).unwrap();
    compiler.declare_function(&testcase.func).unwrap();
    compiler.define_function(testcase.func.clone()).unwrap();
    compiler
        .create_trampoline_for_function(&testcase.func)
        .unwrap();
    let compiled = compiler.compile().unwrap();
    let trampoline = compiled.get_trampoline(&testcase.func).unwrap();

    for args in &testcase.inputs {
        // We rebuild the interpreter every run so that we don't accidentally carry over any state
        // between runs, such as fuel remaining.
        let mut interpreter = build_interpreter(&testcase);
        let int_res = run_in_interpreter(&mut interpreter, args);
        match int_res {
            RunResult::Success(_) => {}
            RunResult::Trap(_) => {
                // We currently ignore inputs that trap the interpreter
                // We could catch traps in the host run and compare them to the
                // interpreter traps, but since we already test trap cases with
                // wasm tests and wasm-level fuzzing, the amount of effort does
                // not justify implementing it again here.
                return;
            }
            RunResult::Timeout => {
                // We probably generated an infinite loop, we can ignore this
                return;
            }
            RunResult::Error(_) => panic!("interpreter failed: {:?}", int_res),
        }

        let host_res = run_in_host(&trampoline, args);
        match host_res {
            RunResult::Success(_) => {}
            _ => panic!("host failed: {:?}", host_res),
        }

        assert_eq!(int_res.unwrap(), host_res.unwrap());
    }
});
