//! File tests.
//!
//! This crate contains the main test driver as well as implementations of the
//! available filetest commands.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::map_unwrap_or,
        clippy::clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

pub use crate::function_runner::SingleFunctionCompiler;
use crate::runner::TestRunner;
use cranelift_codegen::timing;
use cranelift_reader::TestCommand;
use std::path::Path;
use std::time;

mod concurrent;
pub mod function_runner;
mod match_directive;
mod runner;
mod runone;
mod subtest;

mod test_binemit;
mod test_cat;
mod test_compile;
mod test_dce;
mod test_domtree;
mod test_interpret;
mod test_legalizer;
mod test_licm;
mod test_peepmatic;
mod test_postopt;
mod test_preopt;
mod test_print_cfg;
mod test_regalloc;
mod test_rodata;
mod test_run;
mod test_safepoint;
mod test_shrink;
mod test_simple_gvn;
mod test_simple_preopt;
mod test_stack_maps;
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
pub fn run(verbose: bool, report_times: bool, files: &[String]) -> anyhow::Result<time::Duration> {
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
) -> anyhow::Result<time::Duration> {
    let mut runner = TestRunner::new(verbose, /* report_times */ false);

    let path = Path::new(file);
    if path == Path::new("-") || path.is_file() {
        runner.push_test(path);
    } else {
        runner.push_dir(path);
    }

    let result = runner.run_passes(passes, target);
    if report_times {
        println!("{}", timing::take_current());
    }
    result
}

/// Create a new subcommand trait object to match `parsed.command`.
///
/// This function knows how to create all of the possible `test <foo>` commands that can appear in
/// a `.clif` test file.
fn new_subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn subtest::SubTest>> {
    match parsed.command {
        "binemit" => test_binemit::subtest(parsed),
        "cat" => test_cat::subtest(parsed),
        "compile" => test_compile::subtest(parsed),
        "dce" => test_dce::subtest(parsed),
        "domtree" => test_domtree::subtest(parsed),
        "interpret" => test_interpret::subtest(parsed),
        "legalizer" => test_legalizer::subtest(parsed),
        "licm" => test_licm::subtest(parsed),
        "peepmatic" => test_peepmatic::subtest(parsed),
        "postopt" => test_postopt::subtest(parsed),
        "preopt" => test_preopt::subtest(parsed),
        "print-cfg" => test_print_cfg::subtest(parsed),
        "regalloc" => test_regalloc::subtest(parsed),
        "rodata" => test_rodata::subtest(parsed),
        "run" => test_run::subtest(parsed),
        "safepoint" => test_safepoint::subtest(parsed),
        "shrink" => test_shrink::subtest(parsed),
        "simple-gvn" => test_simple_gvn::subtest(parsed),
        "simple_preopt" => test_simple_preopt::subtest(parsed),
        "stack_maps" => test_stack_maps::subtest(parsed),
        "unwind" => test_unwind::subtest(parsed),
        "verifier" => test_verifier::subtest(parsed),
        _ => anyhow::bail!("unknown test command '{}'", parsed.command),
    }
}

fn pretty_anyhow_error(
    func: &cranelift_codegen::ir::Function,
    isa: Option<&dyn cranelift_codegen::isa::TargetIsa>,
    err: cranelift_codegen::CodegenError,
) -> anyhow::Error {
    let s = cranelift_codegen::print_errors::pretty_error(func, isa, err);
    anyhow::anyhow!("{}", s)
}
