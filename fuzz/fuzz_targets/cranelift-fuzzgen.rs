#![no_main]

use libfuzzer_sys::fuzz_target;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{LibCall, TrapCode};
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
use smallvec::smallvec;

const INTERPRETER_FUEL: u64 = 4096;

/// Gather statistics about the fuzzer executions
struct Statistics {
    /// Inputs that fuzzgen can build a function with
    /// This is also how many compiles we executed
    pub valid_inputs: AtomicU64,

    /// Total amount of runs that we tried in the interpreter
    /// One fuzzer input can have many runs
    pub total_runs: AtomicU64,
    /// How many runs were successful?
    /// This is also how many runs were run in the backend
    pub run_result_success: AtomicU64,
    /// How many runs resulted in a timeout?
    pub run_result_timeout: AtomicU64,
    /// How many runs ended with a trap?
    pub run_result_trap: HashMap<CraneliftTrap, AtomicU64>,
}

impl Statistics {
    pub fn print(&self, valid_inputs: u64) {
        // We get valid_inputs as a param since we already loaded it previously.
        let total_runs = self.total_runs.load(Ordering::SeqCst);
        let run_result_success = self.run_result_success.load(Ordering::SeqCst);
        let run_result_timeout = self.run_result_timeout.load(Ordering::SeqCst);

        println!("== FuzzGen Statistics  ====================");
        println!("Valid Inputs: {}", valid_inputs);
        println!("Total Runs: {}", total_runs);
        println!(
            "Successful Runs: {} ({:.1}% of Total Runs)",
            run_result_success,
            (run_result_success as f64 / total_runs as f64) * 100.0
        );
        println!(
            "Timed out Runs: {} ({:.1}% of Total Runs)",
            run_result_timeout,
            (run_result_timeout as f64 / total_runs as f64) * 100.0
        );
        println!("Traps:");
        // Load and filter out empty trap codes.
        let mut traps = self
            .run_result_trap
            .iter()
            .map(|(trap, count)| (trap, count.load(Ordering::SeqCst)))
            .filter(|(_, count)| *count != 0)
            .collect::<Vec<_>>();

        // Sort traps by count in a descending order
        traps.sort_by_key(|(_, count)| -(*count as i64));

        for (trap, count) in traps.into_iter() {
            println!(
                "\t{}: {} ({:.1}% of Total Runs)",
                trap,
                count,
                (count as f64 / total_runs as f64) * 100.0
            );
        }
    }
}

impl Default for Statistics {
    fn default() -> Self {
        // Pre-Register all trap codes since we can't modify this hashmap atomically.
        let mut run_result_trap = HashMap::new();
        run_result_trap.insert(CraneliftTrap::Debug, AtomicU64::new(0));
        run_result_trap.insert(CraneliftTrap::Resumable, AtomicU64::new(0));
        for trapcode in TrapCode::non_user_traps() {
            run_result_trap.insert(CraneliftTrap::User(*trapcode), AtomicU64::new(0));
        }

        Self {
            valid_inputs: AtomicU64::new(0),
            total_runs: AtomicU64::new(0),
            run_result_success: AtomicU64::new(0),
            run_result_timeout: AtomicU64::new(0),
            run_result_trap,
        }
    }
}

#[derive(Debug)]
enum RunResult {
    Success(Vec<DataValue>),
    Trap(CraneliftTrap),
    Timeout,
    Error(Box<dyn std::error::Error>),
}

impl PartialEq for RunResult {
    fn eq(&self, other: &Self) -> bool {
        if let (RunResult::Success(l), RunResult::Success(r)) = (self, other) {
            l.len() == r.len() && l.iter().zip(r).all(|(l, r)| l.bitwise_eq(r))
        } else {
            false
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

static STATISTICS: Lazy<Statistics> = Lazy::new(Statistics::default);

fuzz_target!(|testcase: TestCase| {
    // Periodically print statistics
    let valid_inputs = STATISTICS.valid_inputs.fetch_add(1, Ordering::SeqCst);
    if valid_inputs != 0 && valid_inputs % 10000 == 0 {
        STATISTICS.print(valid_inputs);
    }

    // Native fn
    let flags = {
        let mut builder = settings::builder();
        // We need llvm ABI extensions for i128 values on x86
        builder.set("enable_llvm_abi_extensions", "true").unwrap();

        // This is the default, but we should ensure that it wasn't accidentally turned off anywhere.
        builder.set("enable_verifier", "true").unwrap();

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
        STATISTICS.total_runs.fetch_add(1, Ordering::SeqCst);

        // We rebuild the interpreter every run so that we don't accidentally carry over any state
        // between runs, such as fuel remaining.
        let mut interpreter = build_interpreter(&testcase);
        let int_res = run_in_interpreter(&mut interpreter, args);
        match int_res {
            RunResult::Success(_) => {
                STATISTICS.run_result_success.fetch_add(1, Ordering::SeqCst);
            }
            RunResult::Trap(trap) => {
                STATISTICS.run_result_trap[&trap].fetch_add(1, Ordering::SeqCst);
                // If this input traps, skip it and continue trying other inputs
                // for this function. We've already compiled it anyway.
                //
                // We could catch traps in the host run and compare them to the
                // interpreter traps, but since we already test trap cases with
                // wasm tests and wasm-level fuzzing, the amount of effort does
                // not justify implementing it again here.
                continue;
            }
            RunResult::Timeout => {
                // We probably generated an infinite loop, we should drop this entire input.
                // We could `continue` like we do on traps, but timeouts are *really* expensive.
                STATISTICS.run_result_timeout.fetch_add(1, Ordering::SeqCst);
                return;
            }
            RunResult::Error(_) => panic!("interpreter failed: {:?}", int_res),
        }

        let host_res = run_in_host(&trampoline, args);
        match host_res {
            RunResult::Success(_) => {}
            _ => panic!("host failed: {:?}", host_res),
        }

        assert_eq!(int_res, host_res);
    }
});
