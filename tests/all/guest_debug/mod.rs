//! Integration tests for guest-debug support (gdbstub + LLDB).
//!
//! These tests launch `wasmtime run` with `-g <port>`, connect LLDB via
//! the wasm remote protocol, execute debug scripts (set breakpoints,
//! continue, step, inspect variables), and validate output.
//!
//! Requirements:
//!   - LLDB with Wasm plugin support (`LLDB` env var or `/opt/wasi-sdk/bin/lldb` by default)
//!   - `WASI_SDK_PATH` env var set (for C test programs)
//!   - Built with `--features gdbstub`

use filecheck::{CheckerBuilder, NO_VARIABLES};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use test_programs_artifacts::*;
use wasmtime::{Result, bail, format_err};

/// Find the wasmtime binary built alongside the test binary.
fn wasmtime_binary() -> std::path::PathBuf {
    let mut me = std::env::current_exe().expect("current_exe specified");
    me.pop(); // chop off file name
    me.pop(); // chop off `deps`
    if cfg!(target_os = "windows") {
        me.push("wasmtime.exe");
    } else {
        me.push("wasmtime");
    }
    me
}

/// Find an available TCP port by binding to port 0.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Path to the wasm-aware LLDB.
fn lldb_path() -> String {
    std::env::var("LLDB").unwrap_or("/opt/wasi-sdk/bin/lldb".to_string())
}

/// The readiness marker printed by the gdbstub to stderr.
const GDBSTUB_READY_MARKER: &str = "Debugger listening on";

/// A running wasmtime process with a gdbstub endpoint.
struct WasmtimeWithGdbstub {
    child: Child,
    /// Keeps the stderr pipe alive to avoid SIGPIPE on the child.
    #[allow(dead_code)]
    stderr_reader: BufReader<std::process::ChildStderr>,
}

impl WasmtimeWithGdbstub {
    /// Spawn wasmtime and wait for stderr to contain the gdbstub
    /// readiness marker.
    fn spawn(
        subcmd: &str,
        gdbstub_port: u16,
        extra_args: &[&str],
        timeout: Duration,
    ) -> Result<Self> {
        let mut cmd = Command::new(wasmtime_binary());
        cmd.arg(subcmd)
            .arg(format!("-g{gdbstub_port}"))
            .args(extra_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        eprintln!("spawning: {cmd:?}");
        let mut child = cmd.spawn()?;

        let stderr = child.stderr.take().unwrap();
        let mut reader = BufReader::new(stderr);
        let deadline = std::time::Instant::now() + timeout;
        let mut line = String::new();
        loop {
            if std::time::Instant::now() > deadline {
                let _ = child.kill();
                bail!("timed out waiting for gdbstub readiness");
            }
            line.clear();
            reader.read_line(&mut line)?;
            eprintln!("wasmtime stderr: {}", line.trim_end());
            if line.contains(GDBSTUB_READY_MARKER) {
                return Ok(Self {
                    child,
                    stderr_reader: reader,
                });
            }
            if line.is_empty() {
                let _ = child.kill();
                let status = child.wait()?;
                bail!("wasmtime exited ({status}) without readiness marker");
            }
        }
    }
}

/// Run an LLDB debug script against a gdbstub endpoint.
///
/// Connects LLDB to `127.0.0.1:<port>` using the Wasm plugin,
/// executes the given script commands, and returns LLDB's stdout.
fn lldb_with_gdbstub_script(port: u16, script: &str) -> Result<String> {
    let _ = env_logger::try_init();

    let mut cmd = Command::new(lldb_path());
    cmd.arg("--batch");
    cmd.arg("-o").arg(format!(
        "process connect --plugin wasm connect://127.0.0.1:{port}"
    ));
    for line in script.lines() {
        let line = line.trim();
        if !line.is_empty() {
            cmd.arg("-o").arg(line);
        }
    }

    eprintln!("Running LLDB: {cmd:?}");
    let output = cmd.output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    eprintln!("--- LLDB stdout ---\n{stdout}");
    eprintln!("--- LLDB stderr ---\n{stderr}");

    Ok(stdout)
}

/// Validate output against FileCheck-style directives.
fn check_output(output: &str, directives: &str) -> Result<()> {
    let mut builder = CheckerBuilder::new();
    builder
        .text(directives)
        .map_err(|e| format_err!("unable to build checker: {e:?}"))?;
    let checker = builder.finish();
    let check = checker
        .explain(output, NO_VARIABLES)
        .map_err(|e| format_err!("{e:?}"))?;
    assert!(check.0, "didn't pass check {}", check.1);
    Ok(())
}

/// Test that breakpoints can be set at the initial stop (before any
/// continue), then hit when the program runs.
#[test]
#[ignore]
fn guest_debug_cli_fib_breakpoint() -> Result<()> {
    let port = free_port();
    let mut wt = WasmtimeWithGdbstub::spawn(
        "run",
        port,
        &["-Ccache=n", GUEST_DEBUG_FIB],
        Duration::from_secs(30),
    )?;

    // Set breakpoint at the initial stop, *before* continuing.
    // This tests that modules are visible immediately.
    let output = lldb_with_gdbstub_script(
        port,
        r#"
b fib
c
fr v
c
"#,
    )?;
    wt.child.kill().ok();
    wt.child.wait()?;

    check_output(
        &output,
        r#"
check: stop reason
check: fib
check: n =
"#,
    )?;
    Ok(())
}

/// Test single-stepping within fib.
#[test]
#[ignore]
fn guest_debug_cli_fib_step() -> Result<()> {
    let port = free_port();
    let mut wt = WasmtimeWithGdbstub::spawn(
        "run",
        port,
        &["-Ccache=n", GUEST_DEBUG_FIB],
        Duration::from_secs(30),
    )?;

    let output = lldb_with_gdbstub_script(
        port,
        r#"
b fib
c
n
n
n
fr v
c
"#,
    )?;
    wt.child.kill().ok();
    wt.child.wait()?;

    check_output(
        &output,
        r#"
check: stop reason
check: fib
"#,
    )?;
    Ok(())
}
