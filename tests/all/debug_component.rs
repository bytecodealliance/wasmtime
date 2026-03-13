#![cfg(not(miri))]

use crate::cli_tests::get_wasmtime_command;
use test_programs_artifacts::*;
use wasmtime::Result;

fn run_debugger_test(debugger_component: &str, debuggee: &str, test_mode: &str) -> Result<()> {
    let mut cmd = get_wasmtime_command()?;
    cmd.args(&[
        "run",
        "-Ccache=n",
        &format!("-Ddebugger={debugger_component}"),
        &format!("-Darg={test_mode}"),
        "-Dinherit-stderr=y",
        debuggee,
    ]);
    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        wasmtime::bail!(
            "wasmtime failed with status {}\nstderr:\n{stderr}",
            output.status,
        );
    }
    assert!(
        stderr.contains("OK"),
        "expected 'OK' in stderr, got:\n{stderr}"
    );
    Ok(())
}

#[test]
fn debugger_debuggee_simple() -> Result<()> {
    run_debugger_test(
        DEBUGGER_COMPONENT,
        DEBUGGER_DEBUGGEE_SIMPLE_COMPONENT,
        "simple",
    )
}

#[test]
fn debugger_debuggee_loop() -> Result<()> {
    run_debugger_test(DEBUGGER_COMPONENT, DEBUGGER_DEBUGGEE_LOOP_COMPONENT, "loop")
}

macro_rules! assert_test_exists {
    ($name:ident) => {
        #[expect(unused_imports, reason = "here to assert the test exists")]
        use self::$name as _;
    };
}
foreach_debugger!(assert_test_exists);
