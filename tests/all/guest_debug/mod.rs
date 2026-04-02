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
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
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
    /// Also used by serve tests to read the HTTP address.
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

    /// Read stderr lines until one contains `marker`, returning that line.
    fn wait_for_stderr(&mut self, marker: &str, timeout: Duration) -> Result<String> {
        let deadline = std::time::Instant::now() + timeout;
        let mut line = String::new();
        loop {
            if std::time::Instant::now() > deadline {
                bail!("timed out waiting for '{marker}' on stderr");
            }
            line.clear();
            self.stderr_reader.read_line(&mut line)?;
            eprintln!("wasmtime stderr: {}", line.trim_end());
            if line.contains(marker) {
                return Ok(line);
            }
            if line.is_empty() {
                bail!("wasmtime stderr closed before finding '{marker}'");
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

/// Helper: send an HTTP/1.0 request and return the full response.
fn http_request(addr: SocketAddr, path: &str) -> Result<String> {
    let mut tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    tcp.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(tcp, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n")?;
    let mut response = String::new();
    let _ = std::io::Read::read_to_string(&mut tcp, &mut response);
    Ok(response)
}

/// Parse an HTTP serve address from a "Serving HTTP on http://addr/" line.
fn parse_http_addr(line: &str) -> Result<SocketAddr> {
    line.find("127.0.0.1")
        .and_then(|start| {
            let addr = &line[start..];
            let end = addr.find('/')?;
            addr[..end].parse().ok()
        })
        .ok_or_else(|| format_err!("failed to parse HTTP address from: {line}"))
}

/// Start serve under debugger, continue, and send multiple HTTP requests
/// to verify instance reuse works correctly under the debugger.
#[test]
#[ignore]
fn guest_debug_serve_requests() -> Result<()> {
    let gdb_port = free_port();

    let mut wt = WasmtimeWithGdbstub::spawn(
        "serve",
        gdb_port,
        &[
            "-Ccache=n",
            "--addr=127.0.0.1:0",
            "-Scli",
            P2_CLI_SERVE_HELLO_WORLD_COMPONENT,
        ],
        Duration::from_secs(30),
    )?;

    // Connect LLDB in background: just continue to start the HTTP server.
    let lldb_handle = std::thread::spawn(move || lldb_with_gdbstub_script(gdb_port, "c\n"));

    // Wait for the HTTP server to start.
    let line = wt.wait_for_stderr("Serving HTTP", Duration::from_secs(15))?;
    let http_addr = parse_http_addr(&line)?;
    eprintln!("HTTP address: {http_addr}");

    // Send 3 requests to the same instance, verifying instance reuse.
    for i in 1..=3 {
        let resp = http_request(http_addr, "/")?;
        eprintln!("Response {i}: {}", resp.lines().last().unwrap_or(""));
        assert!(
            resp.contains("Hello, WASI!"),
            "request {i}: expected 'Hello, WASI!' in response, got:\n{resp}"
        );
    }

    // Kill wasmtime to unblock LLDB (which is waiting for the process).
    wt.child.kill().ok();
    wt.child.wait()?;

    // Collect LLDB output (it exits once the process is killed).
    let lldb_output = lldb_handle.join().unwrap()?;

    // Verify LLDB connected and the process was running.
    check_output(
        &lldb_output,
        r#"
check: stop reason
check: resuming
"#,
    )?;

    Ok(())
}

/// Start serve under debugger, set a breakpoint on the HTTP handler,
/// send requests, verify breakpoints fire and responses are correct.
/// Tests instance reuse across multiple requests.
#[test]
#[ignore]
fn guest_debug_serve_breakpoint() -> Result<()> {
    let gdb_port = free_port();

    let mut wt = WasmtimeWithGdbstub::spawn(
        "serve",
        gdb_port,
        &[
            "-Ccache=n",
            "--addr=127.0.0.1:0",
            "-Scli",
            P2_CLI_SERVE_HELLO_WORLD_COMPONENT,
        ],
        Duration::from_secs(30),
    )?;

    // LLDB script: set a breakpoint on the incoming-handler Guest::handle,
    // continue to start the server, then for each request: print backtrace
    // at breakpoint and continue. We do this for 3 requests.
    let lldb_handle = std::thread::spawn(move || {
        lldb_with_gdbstub_script(
            gdb_port,
            r#"
rbreak Guest.*handle
c
bt
c
bt
c
bt
c
"#,
        )
    });

    // Wait for the HTTP server to start.
    let line = wt.wait_for_stderr("Serving HTTP", Duration::from_secs(15))?;
    let http_addr = parse_http_addr(&line)?;
    eprintln!("HTTP address: {http_addr}");

    // Send 3 requests. Each one will hit the breakpoint, LLDB prints
    // the backtrace, then continues to let the response through.
    for i in 1..=3 {
        let resp = http_request(http_addr, "/")?;
        eprintln!("Response {i}: {}", resp.lines().last().unwrap_or(""));
        assert!(
            resp.contains("Hello, WASI!"),
            "request {i}: expected 'Hello, WASI!' in response, got:\n{resp}"
        );
    }

    // Kill wasmtime to unblock LLDB.
    wt.child.kill().ok();
    wt.child.wait()?;

    let lldb_output = lldb_handle.join().unwrap()?;

    // Verify LLDB stopped at the breakpoint with the correct function
    // in the backtrace, and that it happened multiple times.
    check_output(
        &lldb_output,
        r#"
check: Guest
check: handle
check: stop reason
check: Guest
check: handle
check: stop reason
check: Guest
check: handle
"#,
    )?;

    Ok(())
}
