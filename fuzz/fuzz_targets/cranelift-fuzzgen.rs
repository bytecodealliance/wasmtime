#![no_main]

use libfuzzer_sys::fuzz_target;

use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::LibCall;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_filetests::function_runner::{CompiledFunction, SingleFunctionCompiler};
use cranelift_fuzzgen::*;
use cranelift_interpreter::environment::FuncIndex;
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{Interpreter, InterpreterError, InterpreterState};
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

fn run_in_host(compiled_fn: &CompiledFunction, args: &[DataValue]) -> RunResult {
    let res = compiled_fn.call(args);
    RunResult::Success(res)
}

fn interp_libcall_handler(
    libcall: LibCall,
    args: SmallVec<[DataValue; 1]>,
) -> SmallVec<[DataValue; 1]> {
    use LibCall::*;

    smallvec![match (libcall, &args[..]) {
        (CeilF32, [DataValue::F32(a)]) => DataValue::F32(a.ceil()),
        (CeilF64, [DataValue::F64(a)]) => DataValue::F64(a.ceil()),
        (FloorF32, [DataValue::F32(a)]) => DataValue::F32(a.floor()),
        (FloorF64, [DataValue::F64(a)]) => DataValue::F64(a.floor()),
        (TruncF32, [DataValue::F32(a)]) => DataValue::F32(a.trunc()),
        (TruncF64, [DataValue::F64(a)]) => DataValue::F64(a.trunc()),
        _ => unreachable!(),
    }]
}

fn build_interpreter(testcase: &TestCase) -> Interpreter {
    use LibCall::*;

    let mut env = FunctionStore::default();
    env.add(testcase.func.name.to_string(), &testcase.func);

    let state = InterpreterState::default()
        .with_function_store(env)
        .with_libcall(CeilF32, &|args| Ok(interp_libcall_handler(CeilF32, args)))
        .with_libcall(CeilF64, &|args| Ok(interp_libcall_handler(CeilF64, args)))
        .with_libcall(FloorF32, &|args| Ok(interp_libcall_handler(FloorF32, args)))
        .with_libcall(FloorF64, &|args| Ok(interp_libcall_handler(FloorF64, args)))
        .with_libcall(TruncF32, &|args| Ok(interp_libcall_handler(TruncF32, args)))
        .with_libcall(TruncF64, &|args| Ok(interp_libcall_handler(TruncF64, args)));

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
    let host_compiler = SingleFunctionCompiler::with_host_isa(flags).unwrap();
    let compiled_fn = host_compiler.compile(testcase.func.clone()).unwrap();

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

        let host_res = run_in_host(&compiled_fn, args);
        match host_res {
            RunResult::Success(_) => {}
            _ => panic!("host failed: {:?}", host_res),
        }

        assert_eq!(int_res.unwrap(), host_res.unwrap());
    }
});
