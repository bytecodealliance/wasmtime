#![allow(dead_code)]

use anyhow::{bail, format_err, Result};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use std::env;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn lldb_with_script(args: &[&str], script: &str) -> Result<String> {
    let lldb_path = env::var("LLDB").unwrap_or("lldb".to_string());
    let mut cmd = Command::new(&lldb_path);

    cmd.arg("--batch");
    if cfg!(target_os = "macos") {
        cmd.args(&["-o", "settings set plugin.jit-loader.gdb.enable on"]);
    }
    let mut script_file = NamedTempFile::new()?;
    script_file.write(script.as_bytes())?;
    let script_path = script_file.path().to_str().unwrap();
    cmd.args(&["-s", &script_path]);

    let mut me = std::env::current_exe().expect("current_exe specified");
    me.pop(); // chop off the file name
    me.pop(); // chop off `deps`
    if cfg!(target_os = "windows") {
        me.push("wasmtime.exe");
    } else {
        me.push("wasmtime");
    }
    cmd.arg(me);

    cmd.arg("--");
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

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_debug_dwarf_lldb() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "--invoke",
            "fib",
            "tests/all/debug/testsuite/fib-wasm.wasm",
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

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_debug_dwarf5_lldb() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "--invoke",
            "fib",
            "tests/all/debug/testsuite/fib-wasm-dwarf5.wasm",
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

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_debug_dwarf_ref() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Oopt-level=0",
            "-Ddebug-info",
            "tests/all/debug/testsuite/fraction-norm.wasm",
        ],
        r#"b fraction-norm.cc:26
r
p __vmctx->set(),n->denominator
c"#,
    )?;

    check_lldb_output(
        &output,
        r#"
check: Breakpoint 1: no locations (pending)
check: stop reason = breakpoint 1.1
check: frame #0
sameln: norm(n=(__ptr =
check: = 27
check: resuming
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_debug_inst_offsets_are_correct_when_branches_are_removed() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Oopt-level=0",
            "-Ddebug-info",
            "tests/all/debug/testsuite/two_removed_branches.wasm",
        ],
        r#"r"#,
    )?;

    // We are simply checking that the output compiles.
    check_lldb_output(
        &output,
        r#"
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_spilled_frame_base_is_accessible() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Oopt-level=0",
            "-Ddebug-info",
            "tests/all/debug/testsuite/spilled_frame_base.wasm",
        ],
        r#"b spilled_frame_base.c:8
r
fr v i
n
fr v i
n
fr v i
n
fr v i
n
fr v i
n
fr v i
c
"#,
    )?;

    // Check that if the frame base (shadow frame pointer) local
    // is spilled, we can still read locals that reference it.
    check_lldb_output(
        &output,
        r#"
check: i = 0
check: i = 1
check: i = 1
check: i = 1
check: i = 1
check: i = 1
check: exited with status
"#,
    )?;
    Ok(())
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
pub fn test_debug_dwarf5_fission_lldb() -> Result<()> {
    let output = lldb_with_script(
        &[
            "-Ccache=n",
            "-Ddebug-info",
            "tests/all/debug/testsuite/dwarf_fission.wasm",
        ],
        r#"breakpoint set --file 1.c --line 6
r
fr v
s
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
check: i = 1
check: stop reason = step in
check: i = 2
check: resuming
check: exited with status = 0
"#,
    )?;
    Ok(())
}
