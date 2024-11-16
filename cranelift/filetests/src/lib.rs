//! File tests.
//!
//! This crate contains the main test driver as well as implementations of the
//! available filetest commands.

#![deny(missing_docs)]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

pub use crate::function_runner::TestFileCompiler;
use crate::runner::TestRunner;
use cranelift_reader::TestCommand;
use std::path::Path;

mod concurrent;
pub mod function_runner;
mod match_directive;
mod runner;
mod runone;
mod subtest;

mod test_alias_analysis;
mod test_cat;
mod test_compile;
mod test_domtree;
mod test_interpret;
mod test_legalizer;
mod test_optimize;
mod test_print_cfg;
mod test_run;
mod test_safepoint;
mod test_unwind;
mod test_verifier;

/// Main entry point for `clif-util test`.
///
/// Take a list of filenames which can be either `.clif` files or directories.
///
/// Files are interpreted as test cases and executed immediately.
///
/// Directories are scanned recursively for test cases ending in `.clif`. These test cases are
/// executed on background threads.
///
pub fn run(verbose: bool, report_times: bool, files: &[String]) -> anyhow::Result<()> {
    let mut runner = TestRunner::new(verbose, report_times);

    for path in files.iter().map(Path::new) {
        if path.is_file() {
            runner.push_test(path);
        } else {
            runner.push_dir(path);
        }
    }

    runner.start_threads();
    runner.run()
}

/// Used for 'pass' subcommand.
/// Commands are interpreted as test and executed.
///
/// Directories are scanned recursively for test cases ending in `.clif`.
///
pub fn run_passes(
    verbose: bool,
    report_times: bool,
    passes: &[String],
    target: &str,
    file: &str,
) -> anyhow::Result<()> {
    let mut runner = TestRunner::new(verbose, report_times);

    let path = Path::new(file);
    if path == Path::new("-") || path.is_file() {
        runner.push_test(path);
    } else {
        runner.push_dir(path);
    }

    runner.start_threads();
    runner.run_passes(passes, target)
}

/// Create a new subcommand trait object to match `parsed.command`.
///
/// This function knows how to create all of the possible `test <foo>` commands that can appear in
/// a `.clif` test file.
fn new_subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn subtest::SubTest>> {
    match parsed.command {
        "alias-analysis" => test_alias_analysis::subtest(parsed),
        "cat" => test_cat::subtest(parsed),
        "compile" => test_compile::subtest(parsed),
        "domtree" => test_domtree::subtest(parsed),
        "interpret" => test_interpret::subtest(parsed),
        "legalizer" => test_legalizer::subtest(parsed),
        "optimize" => test_optimize::subtest(parsed),
        "print-cfg" => test_print_cfg::subtest(parsed),
        "run" => test_run::subtest(parsed),
        "safepoint" => test_safepoint::subtest(parsed),
        "unwind" => test_unwind::subtest(parsed),
        "verifier" => test_verifier::subtest(parsed),
        _ => anyhow::bail!("unknown test command '{}'", parsed.command),
    }
}

fn pretty_anyhow_error(
    func: &cranelift_codegen::ir::Function,
    err: cranelift_codegen::CodegenError,
) -> anyhow::Error {
    let s = cranelift_codegen::print_errors::pretty_error(func, err);
    anyhow::anyhow!("{}", s)
}
