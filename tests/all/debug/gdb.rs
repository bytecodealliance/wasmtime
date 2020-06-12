#![allow(dead_code)]

use anyhow::{bail, format_err, Result};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use std::env;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn gdb_with_script(args: &[&str], script: &str) -> Result<String> {
    let lldb_path = env::var("GDB").unwrap_or("gdb".to_string());
    let mut cmd = Command::new(&lldb_path);

    cmd.arg("--batch");
    let mut script_file = NamedTempFile::new()?;
    script_file.write(script.as_bytes())?;
    let script_path = script_file.path().to_str().unwrap();
    cmd.args(&["-x", &script_path]);

    cmd.arg("--args");

    let mut me = std::env::current_exe().expect("current_exe specified");
    me.pop(); // chop off the file name
    me.pop(); // chop off `deps`
    me.push("wasmtime");
    cmd.arg(me);

    cmd.args(args);

    let output = cmd.output().expect("success");
    if !output.status.success() {
        bail!(
            "failed to execute {:?}: {}",
            cmd,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn check_gdb_output(output: &str, directives: &str) -> Result<()> {
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

#[test]
#[ignore]
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub fn test_debug_dwarf_gdb() -> Result<()> {
    let output = gdb_with_script(
        &[
            "-g",
            "tests/all/debug/testsuite/fib-wasm.wasm",
            "--invoke",
            "fib",
            "3",
        ],
        r#"set breakpoint pending on
b fib
r
info locals
c"#,
    )?;

    check_gdb_output(
        &output,
        r#"
check: Breakpoint 1 (fib) pending
check: hit Breakpoint 1
sameln: fib (n=3)
check: a = 0
check: b = 0
check: exited normally
"#,
    )?;
    Ok(())
}
