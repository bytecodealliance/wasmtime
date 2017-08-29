//! File tests.
//!
//! This module contains the main driver for `cton-util test` as well as implementations of the
//! available test commands.

use std::path::Path;
use std::time;
use cton_reader::TestCommand;
use CommandResult;
use filetest::runner::TestRunner;

mod concurrent;
mod runner;
mod runone;
mod subtest;

mod test_binemit;
mod test_cat;
mod test_compile;
mod test_domtree;
mod test_legalizer;
mod test_licm;
mod test_preopt;
mod test_print_cfg;
mod test_regalloc;
mod test_simple_gvn;
mod test_verifier;

/// The result of running the test in a file.
type TestResult = Result<time::Duration, String>;

/// Main entry point for `cton-util test`.
///
/// Take a list of filenames which can be either `.cton` files or directories.
///
/// Files are interpreted as test cases and executed immediately.
///
/// Directories are scanned recursively for test cases ending in `.cton`. These test cases are
/// executed on background threads.
///
pub fn run(verbose: bool, files: Vec<String>) -> CommandResult {
    let mut runner = TestRunner::new(verbose);

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

/// Create a new subcommand trait object to match `parsed.command`.
///
/// This function knows how to create all of the possible `test <foo>` commands that can appear in
/// a `.cton` test file.
fn new_subtest(parsed: &TestCommand) -> subtest::Result<Box<subtest::SubTest>> {
    match parsed.command {
        "binemit" => test_binemit::subtest(parsed),
        "cat" => test_cat::subtest(parsed),
        "compile" => test_compile::subtest(parsed),
        "domtree" => test_domtree::subtest(parsed),
        "legalizer" => test_legalizer::subtest(parsed),
        "licm" => test_licm::subtest(parsed),
        "preopt" => test_preopt::subtest(parsed),
        "print-cfg" => test_print_cfg::subtest(parsed),
        "regalloc" => test_regalloc::subtest(parsed),
        "simple-gvn" => test_simple_gvn::subtest(parsed),
        "verifier" => test_verifier::subtest(parsed),
        _ => Err(format!("unknown test command '{}'", parsed.command)),
    }
}
