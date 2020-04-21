use anyhow::{bail, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::NamedTempFile;

// Run the wasmtime CLI with the provided args and return the `Output`.
fn run_wasmtime_for_output(args: &[&str]) -> Result<Output> {
    let mut me = std::env::current_exe()?;
    me.pop(); // chop off the file name
    me.pop(); // chop off `deps`
    me.push("wasmtime");
    Command::new(&me).args(args).output().map_err(Into::into)
}

// Run the wasmtime CLI with the provided args and, if it succeeds, return
// the standard output in a `String`.
fn run_wasmtime(args: &[&str]) -> Result<String> {
    let output = run_wasmtime_for_output(args)?;
    if !output.status.success() {
        bail!(
            "Failed to execute wasmtime with: {:?}\n{}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout).unwrap())
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
        "--disable-cache",
        "4",
    ])?;
    Ok(())
}

// Wasmtime shakk when not enough arguments were provided.
#[test]
fn run_wasmtime_simple_fail_no_args() -> Result<()> {
    let wasm = build_wasm("tests/wasm/simple.wat")?;
    assert!(
        run_wasmtime(&[
            "run",
            wasm.path().to_str().unwrap(),
            "--disable-cache",
            "--invoke",
            "simple",
        ])
        .is_err(),
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
        "--disable-cache",
        "4",
    ])?;
    Ok(())
}

// Running a wat that traps.
#[test]
fn run_wasmtime_unreachable_wat() -> Result<()> {
    let wasm = build_wasm("tests/wasm/unreachable.wat")?;
    let output = run_wasmtime_for_output(&[wasm.path().to_str().unwrap(), "--disable-cache"])?;

    assert_ne!(output.stderr, b"");
    assert_eq!(output.stdout, b"");
    assert!(!output.status.success());

    let code = output
        .status
        .code()
        .expect("wasmtime process should exit normally");

    // Test for the specific error code Wasmtime uses to indicate a trap return.
    #[cfg(unix)]
    assert_eq!(code, 128 + libc::SIGABRT);
    #[cfg(windows)]
    assert_eq!(code, 3);
    Ok(())
}

// Run a simple WASI hello world, snapshot0 edition.
#[test]
fn hello_wasi_snapshot0() -> Result<()> {
    let wasm = build_wasm("tests/wasm/hello_wasi_snapshot0.wat")?;
    let stdout = run_wasmtime(&[wasm.path().to_str().unwrap(), "--disable-cache"])?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

// Run a simple WASI hello world, snapshot1 edition.
#[test]
fn hello_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/wasm/hello_wasi_snapshot1.wat")?;
    let stdout = run_wasmtime(&[wasm.path().to_str().unwrap(), "--disable-cache"])?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

#[test]
fn timeout_in_start() -> Result<()> {
    let wasm = build_wasm("tests/wasm/iloop-start.wat")?;
    let output = run_wasmtime_for_output(&[
        "run",
        wasm.path().to_str().unwrap(),
        "--wasm-timeout",
        "1ms",
        "--disable-cache",
    ])?;
    assert!(!output.status.success());
    assert_eq!(output.stdout, b"");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wasm trap: interrupt"),
        "bad stderr: {}",
        stderr
    );
    Ok(())
}

#[test]
fn timeout_in_invoke() -> Result<()> {
    let wasm = build_wasm("tests/wasm/iloop-invoke.wat")?;
    let output = run_wasmtime_for_output(&[
        "run",
        wasm.path().to_str().unwrap(),
        "--wasm-timeout",
        "1ms",
        "--disable-cache",
    ])?;
    assert!(!output.status.success());
    assert_eq!(output.stdout, b"");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wasm trap: interrupt"),
        "bad stderr: {}",
        stderr
    );
    Ok(())
}
