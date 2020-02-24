#![allow(dead_code)]

use anyhow::{bail, format_err, Result};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

fn lldb_with_script(wasmtime_path: &str, args: &[&str], script: &str) -> Result<String> {
    let lldb_path = env::var("LLDB").unwrap_or("lldb".to_string());
    let mut script_file = NamedTempFile::new()?;
    script_file.write(script.as_bytes())?;
    let script_path = script_file.path().to_str().unwrap();
    let mut lldb_args = vec!["--batch"];
    if cfg!(target_os = "macos") {
        lldb_args.extend_from_slice(&["-o", "settings set plugin.jit-loader.gdb.enable on"]);
    }
    lldb_args.extend_from_slice(&["-s", &script_path, wasmtime_path, "--"]);
    lldb_args.extend_from_slice(args);
    let output = Command::new(&lldb_path)
        .args(&lldb_args)
        .output()
        .expect("success");
    if !output.status.success() {
        bail!(
            "failed to execute {}: {}",
            lldb_path,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn check_lldb_output(output: &str, directives: &str) -> Result<()> {
    let mut builder = CheckerBuilder::new();
    builder
        .text(directives)
        .map_err(|e| format_err!("unable to build checker: {:?}", e))?;
    let checker = builder.finish();
    let check = checker
        .explain(output, NO_VARIABLES)
        .map_err(|e| format_err!("{:?}", e))?;
    assert!(check.0, "didn't pass check {}", check.1);
    Ok(())
}

fn build_wasmtime() -> Result<String> {
    let cargo = env::var("CARGO").unwrap_or("cargo".to_string());
    let pkg_dir = env!("CARGO_MANIFEST_DIR");
    let success = Command::new(cargo)
        .current_dir(pkg_dir)
        .stdout(Stdio::null())
        .args(if cfg!(debug_assertions) {
            ["build"].iter()
        } else {
            ["build", "--release"].iter()
        })
        .args(&["--bin", "wasmtime"])
        .status()?
        .success();
    if !success {
        bail!("Failed to build wasmtime");
    }
    Ok(if cfg!(debug_assertions) {
        "target/debug/wasmtime"
    } else {
        "target/release/wasmtime"
    }
    .to_string())
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_debug_dwarf_lldb() -> Result<()> {
    let wasmtime_path = build_wasmtime()?;
    let output = lldb_with_script(
        &wasmtime_path,
        &[
            "-g",
            "tests/debug/testsuite/fib-wasm.wasm",
            "--invoke",
            "fib",
            "3",
        ],
        r#"b fib
r
fr v
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: Unable to resolve breakpoint to any actual locations.
check: 1 location added to breakpoint 1
check: stop reason = breakpoint 1.1
check: frame #0
sameln: JIT
sameln: fib(n=3)
check: n = 3
check: a = 0
check: resuming
check: exited with status
"#,
    )?;
    Ok(())
}
