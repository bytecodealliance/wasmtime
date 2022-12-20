#![no_main]

use libfuzzer_sys::fuzz_target;
use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use cranelift_filetests::function_runner::TestFileCompiler;
use cranelift_fuzzgen::*;

/// Gather statistics about the fuzzer executions
struct Statistics {
    /// Total amount of runs that we tried to compile.
    pub total_runs: AtomicU64,

    /// How many runs were successful?
    /// This is also how many runs were run in the backend
    pub run_result_success: AtomicU64,
}

impl Statistics {
    pub fn print(&self, total_runs: u64) {
        // We get valid_inputs as a param since we already loaded it previously.
        let run_result_success = self.run_result_success.load(Ordering::SeqCst);

        println!("== FuzzGen Statistics  ====================");
        println!("Total Runs: {}", total_runs);
        println!(
            "Successful Runs: {} ({:.1}% of Total Runs)",
            run_result_success,
            (run_result_success as f64 / total_runs as f64) * 100.0
        );
    }
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            total_runs: AtomicU64::new(0),
            run_result_success: AtomicU64::new(0),
        }
    }
}

static STATISTICS: Lazy<Statistics> = Lazy::new(Statistics::default);

fuzz_target!(|testcase: FunctionWithIsa| {
    // This is the default, but we should ensure that it wasn't accidentally turned off anywhere.
    assert!(testcase.isa.flags().enable_verifier());

    let total_inputs = STATISTICS.total_runs.fetch_add(1, Ordering::SeqCst);

    // Periodically print statistics
    if total_inputs != 0 && total_inputs % 10000 == 0 {
        STATISTICS.print(total_inputs);
    }

    let mut compiler = TestFileCompiler::new(testcase.isa);
    compiler.declare_function(&testcase.func).unwrap();
    compiler.define_function(testcase.func.clone()).unwrap();
    compiler
        .create_trampoline_for_function(&testcase.func)
        .unwrap();

    if compiler.compile().is_err() {
        panic!("failed to compile input");
    }

    STATISTICS.run_result_success.fetch_add(1, Ordering::SeqCst);
});
