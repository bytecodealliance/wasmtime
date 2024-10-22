#![no_main]

use cranelift_codegen::ir::Function;
use cranelift_codegen::ir::Signature;
use cranelift_codegen::ir::UserExternalName;
use cranelift_codegen::ir::UserFuncName;
use cranelift_codegen::Context;
use cranelift_control::ControlPlane;
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;

use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{LibCall, TrapCode};
use cranelift_codegen::isa;
use cranelift_filetests::function_runner::{TestFileCompiler, Trampoline};
use cranelift_fuzzgen::*;
use cranelift_interpreter::environment::FuncIndex;
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{
    Interpreter, InterpreterError, InterpreterState, LibCallValues,
};
use cranelift_interpreter::step::ControlFlow;
use cranelift_interpreter::step::CraneliftTrap;
use cranelift_native::builder_with_options;
use smallvec::smallvec;

const INTERPRETER_FUEL: u64 = 4096;

/// Gather statistics about the fuzzer executions
struct Statistics {
    /// Inputs that fuzzgen can build a function with
    /// This is also how many compiles we executed
    pub valid_inputs: AtomicU64,
    /// How many times did we generate an invalid format?
    pub invalid_inputs: AtomicU64,

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
        let invalid_inputs = self.invalid_inputs.load(Ordering::SeqCst);
        let run_result_success = self.run_result_success.load(Ordering::SeqCst);
        let run_result_timeout = self.run_result_timeout.load(Ordering::SeqCst);

        println!("== FuzzGen Statistics  ====================");
        println!("Valid Inputs: {valid_inputs}");
        println!(
            "Invalid Inputs: {} ({:.1}% of Total Inputs)",
            invalid_inputs,
            (invalid_inputs as f64 / (valid_inputs + invalid_inputs) as f64) * 100.0
        );
        println!("Total Runs: {total_runs}");
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
        run_result_trap.insert(CraneliftTrap::BadSignature, AtomicU64::new(0));
        run_result_trap.insert(CraneliftTrap::UnreachableCodeReached, AtomicU64::new(0));
        run_result_trap.insert(CraneliftTrap::HeapMisaligned, AtomicU64::new(0));
        for trapcode in TrapCode::non_user_traps() {
            run_result_trap.insert(CraneliftTrap::User(*trapcode), AtomicU64::new(0));
        }

        Self {
            valid_inputs: AtomicU64::new(0),
            invalid_inputs: AtomicU64::new(0),
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
        match (self, other) {
            (RunResult::Success(l), RunResult::Success(r)) => {
                l.len() == r.len() && l.iter().zip(r).all(|(l, r)| l.bitwise_eq(r))
            }
            (RunResult::Trap(l), RunResult::Trap(r)) => l == r,
            (RunResult::Timeout, RunResult::Timeout) => true,
            (RunResult::Error(_), RunResult::Error(_)) => unimplemented!(),
            _ => false,
        }
    }
}

pub struct TestCase {
    /// TargetIsa to use when compiling this test case
    pub isa: isa::OwnedTargetIsa,
    /// Functions under test
    /// By convention the first function is the main function.
    pub functions: Vec<Function>,
    /// Control planes for function compilation.
    /// There should be an equal amount as functions to compile.
    pub ctrl_planes: Vec<ControlPlane>,
    /// Generate multiple test inputs for each test case.
    /// This allows us to get more coverage per compilation, which may be somewhat expensive.
    pub inputs: Vec<TestCaseInput>,
    /// Should this `TestCase` be tested after optimizations.
    pub compare_against_host: bool,
}

impl fmt::Debug for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.compare_against_host {
            writeln!(f, ";; Testing against optimized version")?;
        }
        PrintableTestCase::run(&self.isa, &self.functions, &self.inputs).fmt(f)
    }
}

impl<'a> Arbitrary<'a> for TestCase {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let _ = env_logger::try_init();
        Self::generate(u).map_err(|_| {
            STATISTICS.invalid_inputs.fetch_add(1, Ordering::SeqCst);
            arbitrary::Error::IncorrectFormat
        })
    }
}

impl TestCase {
    pub fn generate(u: &mut Unstructured) -> anyhow::Result<Self> {
        let mut gen = FuzzGen::new(u);

        let compare_against_host = gen.u.arbitrary()?;

        // TestCase is meant to be consumed by a runner, so we make the assumption here that we're
        // generating a TargetIsa for the host.
        let mut builder =
            builder_with_options(true).expect("Unable to build a TargetIsa for the current host");
        let flags = gen.generate_flags(builder.triple().architecture)?;
        gen.set_isa_flags(&mut builder, IsaFlagGen::Host)?;
        let isa = builder.finish(flags)?;

        // When generating functions, we allow each function to call any function that has
        // already been generated. This guarantees that we never have loops in the call graph.
        // We generate these backwards, and then reverse them so that the main function is at
        // the start.
        let func_count = gen.u.int_in_range(gen.config.testcase_funcs.clone())?;
        let mut functions: Vec<Function> = Vec::with_capacity(func_count);
        let mut ctrl_planes: Vec<ControlPlane> = Vec::with_capacity(func_count);
        for i in (0..func_count).rev() {
            // Function name must be in a different namespace than TESTFILE_NAMESPACE (0)
            let fname = UserFuncName::user(1, i as u32);

            let usercalls: Vec<(UserExternalName, Signature)> = functions
                .iter()
                .map(|f| {
                    (
                        f.name.get_user().unwrap().clone(),
                        f.stencil.signature.clone(),
                    )
                })
                .collect();

            let func =
                gen.generate_func(fname, isa.clone(), usercalls, ALLOWED_LIBCALLS.to_vec())?;
            functions.push(func);

            ctrl_planes.push(ControlPlane::arbitrary(gen.u)?);
        }
        // Now reverse the functions so that the main function is at the start.
        functions.reverse();

        let main = &functions[0];
        let inputs = gen.generate_test_inputs(&main.signature)?;

        Ok(TestCase {
            isa,
            functions,
            ctrl_planes,
            inputs,
            compare_against_host,
        })
    }

    fn to_optimized(&self) -> Self {
        let mut ctrl_planes = self.ctrl_planes.clone();
        let optimized_functions: Vec<Function> = self
            .functions
            .iter()
            .zip(ctrl_planes.iter_mut())
            .map(|(func, ctrl_plane)| {
                let mut ctx = Context::for_function(func.clone());
                ctx.optimize(self.isa.as_ref(), ctrl_plane).unwrap();
                ctx.func
            })
            .collect();

        TestCase {
            isa: self.isa.clone(),
            functions: optimized_functions,
            ctrl_planes,
            inputs: self.inputs.clone(),
            compare_against_host: false,
        }
    }

    /// Returns the main function of this test case.
    pub fn main(&self) -> &Function {
        &self.functions[0]
    }
}

fn run_in_interpreter(interpreter: &mut Interpreter, args: &[DataValue]) -> RunResult {
    // The entrypoint function is always 0
    let index = FuncIndex::from_u32(0);
    let res = interpreter.call_by_index(index, args);

    match res {
        Ok(ControlFlow::Return(results)) => RunResult::Success(results.to_vec()),
        Ok(ControlFlow::Trap(trap)) => RunResult::Trap(trap),
        Ok(cf) => RunResult::Error(format!("Unrecognized exit ControlFlow: {cf:?}").into()),
        Err(InterpreterError::FuelExhausted) => RunResult::Timeout,
        Err(e) => RunResult::Error(e.into()),
    }
}

fn run_in_host(trampoline: &Trampoline, args: &[DataValue]) -> RunResult {
    let res = trampoline.call(args);
    RunResult::Success(res)
}

/// These libcalls need a interpreter implementation in `build_interpreter`
const ALLOWED_LIBCALLS: &'static [LibCall] = &[
    LibCall::CeilF32,
    LibCall::CeilF64,
    LibCall::FloorF32,
    LibCall::FloorF64,
    LibCall::TruncF32,
    LibCall::TruncF64,
];

fn build_interpreter(testcase: &TestCase) -> Interpreter {
    let mut env = FunctionStore::default();
    for func in testcase.functions.iter() {
        env.add(func.name.to_string(), &func);
    }

    let state = InterpreterState::default()
        .with_function_store(env)
        .with_libcall_handler(|libcall: LibCall, args: LibCallValues| {
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

static STATISTICS: LazyLock<Statistics> = LazyLock::new(Statistics::default);

fn run_test_inputs(testcase: &TestCase, run: impl Fn(&[DataValue]) -> RunResult) {
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
            RunResult::Error(e) => panic!("interpreter failed: {e:?}"),
        }

        let res = run(args);

        // This situation can happen when we are comparing the interpreter against the interpreter, and
        // one of the optimization passes has increased the number of instructions in the function.
        // This can cause the interpreter to run out of fuel in the second run, but not the first.
        // We should ignore these cases.
        // Running in the host should never return a timeout, so that should be ok.
        if res == RunResult::Timeout {
            return;
        }

        assert_eq!(int_res, res);
    }
}

fuzz_target!(|testcase: TestCase| {
    let mut testcase = testcase;
    let fuel: u8 = std::env::args()
        .find_map(|arg| arg.strip_prefix("--fuel=").map(|s| s.to_owned()))
        .map(|fuel| fuel.parse().expect("fuel should be a valid integer"))
        .unwrap_or_default();
    for i in 0..testcase.ctrl_planes.len() {
        testcase.ctrl_planes[i].set_fuel(fuel)
    }
    let testcase = testcase;

    // This is the default, but we should ensure that it wasn't accidentally turned off anywhere.
    assert!(testcase.isa.flags().enable_verifier());

    // Periodically print statistics
    let valid_inputs = STATISTICS.valid_inputs.fetch_add(1, Ordering::SeqCst);
    if valid_inputs != 0 && valid_inputs % 10000 == 0 {
        STATISTICS.print(valid_inputs);
    }

    if !testcase.compare_against_host {
        let opt_testcase = testcase.to_optimized();

        run_test_inputs(&testcase, |args| {
            // We rebuild the interpreter every run so that we don't accidentally carry over any state
            // between runs, such as fuel remaining.
            let mut interpreter = build_interpreter(&opt_testcase);

            run_in_interpreter(&mut interpreter, args)
        });
    } else {
        let mut compiler = TestFileCompiler::new(testcase.isa.clone());
        compiler
            .add_functions(&testcase.functions[..], testcase.ctrl_planes.clone())
            .unwrap();
        let compiled = compiler.compile().unwrap();
        let trampoline = compiled.get_trampoline(testcase.main()).unwrap();

        run_test_inputs(&testcase, |args| run_in_host(&trampoline, args));
    }
});
