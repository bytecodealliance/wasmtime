#![no_main]

use libfuzzer_sys::{arbitrary, fuzz_target};
use once_cell::sync::Lazy;
use std::fmt;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use target_lexicon::Architecture;

use cranelift_codegen::ir;
use cranelift_codegen::isa;
use cranelift_codegen::settings;
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

struct CompileTest {
    isa: Box<dyn isa::TargetIsa>,
    func: ir::Function,
}

impl fmt::Debug for CompileTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, ";; Compile test case\n")?;

        // Print only non default flags
        let default_flags = settings::Flags::new(settings::builder());
        for (default, flag) in default_flags.iter().zip(self.isa.flags().iter()) {
            assert_eq!(default.name, flag.name);

            if default.value_string() != flag.value_string() {
                writeln!(f, "set {}={}", flag.name, flag.value_string())?;
            }
        }

        writeln!(f, "test compile\n")?;

        match self.isa.triple().architecture {
            Architecture::X86_64 => writeln!(f, "target aarch64")?,
            Architecture::Aarch64 { .. } => writeln!(f, "target aarch64")?,
            Architecture::S390x => writeln!(f, "target s390x")?,
            Architecture::Riscv64 { .. } => writeln!(f, "target riscv64")?,
            _ => unreachable!(),
        }

        writeln!(f, "{}", self.func)?;

        Ok(())
    }
}

impl<'a> arbitrary::Arbitrary<'a> for CompileTest {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let target = u.choose(&["x86_64", "s390x", "aarch64", "riscv64"])?;
        let builder = isa::lookup_by_name(target).expect("Unable to find target architecture");

        let mut gen = FuzzGen::new(u);
        let flags = gen
            .generate_flags(builder.triple().architecture)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;
        let isa = builder
            .finish(flags)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        let func = gen
            .generate_func()
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;

        Ok(CompileTest { isa, func })
    }
}

fuzz_target!(|testcase: CompileTest| {
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
