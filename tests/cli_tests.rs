use anyhow::{bail, Result};
use std::env;
use std::process::{Command, Stdio};

fn run_wasmtime(args: &[&'static str]) -> Result<()> {
    let cargo = env::var("CARGO").unwrap_or("cargo".to_string());
    let pkg_dir = env!("CARGO_MANIFEST_DIR");
    let success = Command::new(cargo)
        .current_dir(pkg_dir)
        .stdout(Stdio::null())
        .args(&["run", "-q", "--"])
        .args(args)
        .status()?
        .success();
    if !success {
        bail!("Failed to execute wasmtime with: {:?}", args);
    }
    Ok(())
}

// Very basic use case: compile binary wasm file and run specific function with arguments.
#[test]
fn run_wasmtime_simple() -> Result<()> {
    run_wasmtime(&["run", "tests/wasm/simple.wasm", "--invoke", "simple", "4"])
}

// Wasmtime shakk when not enough arguments were provided.
#[test]
fn run_wasmtime_simple_fail_no_args() {
    assert!(
        run_wasmtime(&["run", "tests/wasm/simple.wasm", "--invoke", "simple"]).is_err(),
        "shall fail"
    );
}

// Running simple wat
#[test]
fn run_wasmtime_simple_wat() -> Result<()> {
    run_wasmtime(&["run", "tests/wasm/simple.wat", "--invoke", "simple", "4"])
}
