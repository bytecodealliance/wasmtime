//! Run the tests in `wasi_testsuite` using Wasmtime's CLI binary and checking
//! the results with a [wasi-testsuite] spec.
//!
//! [wasi-testsuite]: https://github.com/WebAssembly/wasi-testsuite

#![cfg(not(miri))]

use crate::cli_tests::get_wasmtime_command;
use anyhow::{anyhow, Result};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use walkdir::{DirEntry, WalkDir};

#[test]
#[cfg_attr(target_os = "windows", ignore)] // TODO: https://github.com/WebAssembly/WASI/issues/524
fn wasi_testsuite() -> Result<()> {
    // Currently, Wasmtime's implementation in wasi-common does not line up
    // exactly with the expectations in wasi-testsuite. This could be for one of
    // various reasons:
    //  - wasi-common has a bug
    //  - wasi-testsuite overspecifies (or incorrectly specifies) a test
    //  - this test runner incorrectly configures Wasmtime's CLI execution.
    //
    // This list is expected to shrink as the failures are resolved. The easiest
    // way to resolve one of these is to remove the file from the list and run
    // `cargo test wasi_testsuite -- --nocapture`. The printed output will show
    // the expected result, the actual result, and a command to replicate the
    // failure from the command line.
    const WASI_COMMON_IGNORE_LIST: &[&str] = &[
        // Uses functions not supported in Wasmtime due to portability concerns
        "fd_advise.wasm",
        "file_allocate.wasm",
        // #8828
        "remove_directory_trailing_slashes.wasm",
        // Working with rights which are removed from wasmtime
        "truncation_rights.wasm",
        "fd_fdstat_set_rights.wasm",
        "path_open_preopen.wasm",
        "path_link.wasm",
        // This test wants an operation to fail that wasmtime thinks is valid
        "renumber.wasm",
        // Works with FDFLAGS_SYNC which isn't supported
        "path_filestat.wasm",
        // this test asserts semantics of pwrite which don't match Linux so
        // ignore the test for now.
        "pwrite-with-append.wasm",
    ];
    run_all(
        "tests/wasi_testsuite/wasi-common",
        &[],
        WASI_COMMON_IGNORE_LIST,
    )?;
    run_all(
        "tests/wasi_testsuite/wasi-threads",
        &["-Sthreads", "-Wthreads"],
        &[],
    )?;
    Ok(())
}

fn run_all(testsuite_dir: &str, extra_flags: &[&str], ignore: &[&str]) -> Result<()> {
    // In case the previous run ended in failure, we clean up any created files
    // that would otherwise be cleaned up at the end of this function.
    clean_garbage(testsuite_dir)?;

    // Execute and check each WebAssembly test case.
    for module in list_files(testsuite_dir, is_wasm) {
        if should_ignore(&module, ignore) {
            println!("Ignoring {}", module.display());
        } else {
            println!("Testing {}", module.display());
            let spec = if let Ok(contents) = fs::read_to_string(&module.with_extension("json")) {
                serde_json::from_str(&contents)?
            } else {
                Spec::default()
            };
            let mut cmd = build_command(module, extra_flags, &spec)?;
            let result = cmd.output()?;
            if spec != result {
                println!("  command: {cmd:?}");
                println!("  spec: {spec:#?}");
                println!("  result.status: {}", result.status);
                if !result.stdout.is_empty() {
                    println!(
                        "  result.stdout:\n    {}",
                        String::from_utf8_lossy(&result.stdout).replace("\n", "\n    ")
                    );
                }
                if !result.stderr.is_empty() {
                    println!(
                        "  result.stderr:\n    {}",
                        String::from_utf8_lossy(&result.stderr).replace("\n", "\n    ")
                    );
                }
                panic!("FAILED! The result does not match the specification");
            }
        }
    }

    // Clean up any created files to avoid making the Git repository dirty.
    clean_garbage(testsuite_dir)
}

fn list_files<F>(testsuite_dir: &str, filter: F) -> impl Iterator<Item = PathBuf>
where
    F: FnMut(&DirEntry) -> bool,
{
    WalkDir::new(testsuite_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(filter)
        .map(|e| e.path().to_path_buf())
}

fn is_wasm(entry: &DirEntry) -> bool {
    let path = entry.path();
    let ext = path.extension().map(OsStr::to_str).flatten();
    path.is_file() && (ext == Some("wat") || ext == Some("wasm"))
}

fn should_ignore<P: AsRef<Path>>(path: P, ignore_list: &[&str]) -> bool {
    let file_name = path.as_ref().file_name().unwrap().to_str().unwrap();
    ignore_list.contains(&file_name)
}

fn build_command<P: AsRef<Path>>(module: P, extra_flags: &[&str], spec: &Spec) -> Result<Command> {
    let mut cmd = get_wasmtime_command()?;
    let parent_dir = module
        .as_ref()
        .parent()
        .ok_or(anyhow!("module has no parent?"))?;

    // Add arguments.
    cmd.args(["run", "-Ccache=n"]);
    cmd.args(extra_flags);
    if let Some(dirs) = &spec.dirs {
        for dir in dirs {
            cmd.arg("--dir");
            cmd.arg(format!("{}::{}", parent_dir.join(dir).display(), dir));
        }
    }
    // Add environment variables as CLI arguments.
    if let Some(env) = &spec.env {
        for env_pair in env {
            cmd.arg("--env");
            cmd.arg(format!("{}={}", env_pair.0, env_pair.1));
        }
        cmd.envs(env);
    }
    cmd.arg(module.as_ref().to_str().unwrap());
    if let Some(spec_args) = &spec.args {
        cmd.args(spec_args);
    }

    Ok(cmd)
}

fn clean_garbage(testsuite_dir: &str) -> Result<()> {
    for path in list_files(testsuite_dir, is_garbage) {
        println!("Removing {}", path.display());
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn is_garbage(entry: &DirEntry) -> bool {
    let path = entry.path();
    let ext = path.extension().map(OsStr::to_str).flatten();
    ext == Some("cleanup")
}

#[derive(Debug, Default, Deserialize)]
struct Spec {
    args: Option<Vec<String>>,
    dirs: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    exit_code: Option<i32>,
    stderr: Option<String>,
    stdout: Option<String>,
}

impl PartialEq<Output> for Spec {
    fn eq(&self, other: &Output) -> bool {
        self.exit_code.unwrap_or(0) == other.status.code().unwrap()
            && matches_or_missing(&self.stdout, &other.stdout)
            && matches_or_missing(&self.stderr, &other.stderr)
    }
}

fn matches_or_missing(a: &Option<String>, b: &[u8]) -> bool {
    a.as_ref()
        .map(|s| s == &String::from_utf8_lossy(b))
        .unwrap_or(true)
}
