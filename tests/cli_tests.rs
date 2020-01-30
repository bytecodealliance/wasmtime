use anyhow::{bail, Result};
use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

fn run_wasmtime(args: &[&str]) -> Result<()> {
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

fn build_wasm(wat_path: impl AsRef<Path>) -> Result<NamedTempFile> {
    let mut wasm_file = NamedTempFile::new()?;
    let wasm = wat::parse_file(wat_path)?;
    wasm_file.write(&wasm)?;
    Ok(wasm_file)
}

// Very basic use case: compile binary wasm file and run specific function with arguments.
#[test]
fn run_wasmtime_simple() -> Result<()> {
    let wasm = build_wasm("tests/wasm/simple.wat")?;
    run_wasmtime(&[
        "run",
        wasm.path().to_str().unwrap(),
        "--invoke",
        "simple",
        "4",
    ])
}

// Wasmtime shakk when not enough arguments were provided.
#[test]
fn run_wasmtime_simple_fail_no_args() -> Result<()> {
    let wasm = build_wasm("tests/wasm/simple.wat")?;
    assert!(
        run_wasmtime(&["run", wasm.path().to_str().unwrap(), "--invoke", "simple"]).is_err(),
        "shall fail"
    );
    Ok(())
}

// Running simple wat
#[test]
fn run_wasmtime_simple_wat() -> Result<()> {
    let wasm = build_wasm("tests/wasm/simple.wat")?;
    run_wasmtime(&[
        "run",
        wasm.path().to_str().unwrap(),
        "--invoke",
        "simple",
        "4",
    ])
}
