use std::path::PathBuf;

use cranelift_isle_veri::runner::{RunFailure, RunSummary, Runner};
use cranelift_isle_veri_test_macros::file_tests;
use tempfile::tempdir;

/// Run the verifier over a single test file, rooted at the `test` term.
///
/// Returns the run summary alongside the overall result, so that a test can
/// assert on the summary of a run that failed.
fn run(test_file: &str) -> anyhow::Result<(RunSummary, anyhow::Result<()>)> {
    let inputs = vec![PathBuf::from(test_file)];
    let mut runner = Runner::from_files(&inputs).expect("should be able to create runner");
    let temp_dir = tempdir().expect("should be able to create temporary log directory");
    runner.set_log_dir(temp_dir.path().join("log"));
    runner.include_first_rule_named();
    runner.set_root_term("test");
    match runner.run() {
        Ok(summary) => Ok((summary, Ok(()))),
        Err(err) => {
            let summary = err.downcast_ref::<RunFailure>().map(|f| f.summary.clone());
            match summary {
                Some(summary) => Ok((summary, Err(err))),
                None => Err(err),
            }
        }
    }
}

#[file_tests(path = "filetests/pass", ext = "isle")]
fn pass(test_file: &str) {
    let (summary, result) = run(test_file).expect("run should complete without errors");
    result.expect("verification should pass");
    assert!(
        summary.success > 0,
        "expected at least one successful verification, got summary {summary:?}"
    );
    assert_eq!(
        summary.spec_conflicts, 0,
        "expected no spec type conflicts, got summary {summary:?}"
    );
}

#[file_tests(path = "filetests/broken", ext = "isle")]
fn broken(test_file: &str) {
    let (_, result) = run(test_file).expect("run should complete without errors");
    result.expect_err("verification should fail");
}

/// Test files containing a term whose spec does not type check on its own.
#[file_tests(path = "filetests/spec_conflict", ext = "isle")]
fn spec_conflict(test_file: &str) {
    let (summary, result) = run(test_file).expect("run should complete without errors");
    result.expect_err("the spec type conflict should fail the run");
    assert!(
        summary.spec_conflicts > 0,
        "expected the spec check to report the conflict directly, rather than only as a missing instantiation, got summary {summary:?}"
    );
    // The run must stop at the check, before verifying anything: a spec that
    // cannot be instantiated would otherwise silently shrink the query set.
    assert_eq!(
        summary.total_instantiations, 0,
        "expected the run to fail before verification, got summary {summary:?}"
    );
    assert_eq!(
        summary.applicable, 0,
        "expected the run to fail before verification, got summary {summary:?}"
    );
}
