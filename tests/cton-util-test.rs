//! Run `cton-util test` on all available testcases.

use std::process::{Command, Output};
use std::env;
use std::path::PathBuf;
use std::io::{self, Write};

/// Returns the target directory, where we can find build artifacts
/// and such for the current configuration.
fn get_target_dir() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop(); // chop off exe name
    path.pop(); // chop off deps name
    path
}

#[test]
fn cton_util_test() {
    let mut cmd = Command::new(&get_target_dir().join("cton-util"));
    cmd.arg("test");

    // We have testcases in the following directories:
    cmd.arg("filetests");
    cmd.arg("docs");

    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().unwrap();
    io::stdout().write(&stdout).unwrap();
    io::stderr().write(&stderr).unwrap();
    assert!(status.success(), "failed with exit status {}", status);
}
