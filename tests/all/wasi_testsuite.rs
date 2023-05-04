//! Run the tests in `wasi_testsuite` using Wasmtime's CLI binary and checking
//! the results with a [wasi-testsuite] spec.
//!
//! [wasi-testsuite]: https://github.com/WebAssembly/wasi-testsuite

#![cfg(not(miri))]

use crate::cli_tests::run_wasmtime_for_output;
use anyhow::Result;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::process::Output;

#[test]
#[cfg_attr(target_os = "windows", ignore)] // TODO: https://github.com/WebAssembly/WASI/issues/524
fn wasi_threads_testsuite() -> Result<()> {
    for module in list_modules("tests/wasi_testsuite/wasi-threads/test/testsuite")? {
        println!("Testing {}", module.display());
        let result = run(&module)?;
        let spec = parse_spec(&module.with_extension("json"))?;
        assert_eq!(spec, result);
    }
    Ok(())
}

fn list_modules(testsuite_dir: &str) -> Result<impl Iterator<Item = PathBuf>> {
    Ok(fs::read_dir(testsuite_dir)?
        .filter_map(Result::ok)
        .filter(is_wasm)
        .map(|e| e.path()))
}

fn is_wasm(entry: &DirEntry) -> bool {
    let path = entry.path();
    let ext = path.extension().map(OsStr::to_str).flatten();
    path.is_file() && (ext == Some("wat") || ext == Some("wasm"))
}

fn run<P: AsRef<Path>>(module: P) -> Result<Output> {
    run_wasmtime_for_output(
        &[
            "run",
            "--wasi-modules",
            "experimental-wasi-threads",
            "--wasm-features",
            "threads",
            "--disable-cache",
            module.as_ref().to_str().unwrap(),
        ],
        None,
    )
}

fn parse_spec<P: AsRef<Path>>(spec_file: P) -> Result<Spec> {
    let contents = fs::read_to_string(spec_file)?;
    let spec = serde_json::from_str(&contents)?;
    Ok(spec)
}

#[derive(Debug, Deserialize)]
struct Spec {
    exit_code: i32,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl PartialEq<Output> for Spec {
    fn eq(&self, other: &Output) -> bool {
        self.exit_code == other.status.code().unwrap()
            && matches_or_missing(&self.stdout, &other.stdout)
            && matches_or_missing(&self.stderr, &other.stderr)
    }
}

fn matches_or_missing(a: &Option<String>, b: &[u8]) -> bool {
    a.as_ref()
        .map(|s| s == &String::from_utf8_lossy(b))
        .unwrap_or(true)
}
