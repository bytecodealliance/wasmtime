#![cfg(not(miri))]

use anyhow::{bail, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::{NamedTempFile, TempDir};

// Run the wasmtime CLI with the provided args and return the `Output`.
// If the `stdin` is `Some`, opens the file and redirects to the child's stdin.
pub fn run_wasmtime_for_output(args: &[&str], stdin: Option<&Path>) -> Result<Output> {
    let mut cmd = get_wasmtime_command()?;
    cmd.args(args);
    if let Some(file) = stdin {
        cmd.stdin(File::open(file)?);
    }
    cmd.output().map_err(Into::into)
}

/// Get the Wasmtime CLI as a [Command].
pub fn get_wasmtime_command() -> Result<Command> {
    // Figure out the Wasmtime binary from the current executable.
    let runner = std::env::vars()
        .filter(|(k, _v)| k.starts_with("CARGO_TARGET") && k.ends_with("RUNNER"))
        .next();
    let mut me = std::env::current_exe()?;
    me.pop(); // chop off the file name
    me.pop(); // chop off `deps`
    me.push("wasmtime");

    // If we're running tests with a "runner" then we might be doing something
    // like cross-emulation, so spin up the emulator rather than the tests
    // itself, which may not be natively executable.
    let cmd = if let Some((_, runner)) = runner {
        let mut parts = runner.split_whitespace();
        let mut cmd = Command::new(parts.next().unwrap());
        for arg in parts {
            cmd.arg(arg);
        }
        cmd.arg(&me);
        cmd
    } else {
        Command::new(&me)
    };

    Ok(cmd)
}

// Run the wasmtime CLI with the provided args and, if it succeeds, return
// the standard output in a `String`.
fn run_wasmtime(args: &[&str]) -> Result<String> {
    let output = run_wasmtime_for_output(args, None)?;
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
    let wasm = build_wasm("tests/all/cli_tests/simple.wat")?;
    run_wasmtime(&[
        "run",
        "--invoke",
        "simple",
        "--disable-cache",
        wasm.path().to_str().unwrap(),
        "4",
    ])?;
    Ok(())
}

// Wasmtime shall fail when not enough arguments were provided.
#[test]
fn run_wasmtime_simple_fail_no_args() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/simple.wat")?;
    assert!(
        run_wasmtime(&[
            "run",
            "--disable-cache",
            "--invoke",
            "simple",
            wasm.path().to_str().unwrap(),
        ])
        .is_err(),
        "shall fail"
    );
    Ok(())
}

#[test]
fn run_coredump_smoketest() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/coredump_smoketest.wat")?;
    let coredump_file = NamedTempFile::new()?;
    let coredump_arg = format!("--coredump-on-trap={}", coredump_file.path().display());
    let err = run_wasmtime(&[
        "run",
        "--invoke",
        "a",
        "--disable-cache",
        &coredump_arg,
        wasm.path().to_str().unwrap(),
    ])
    .unwrap_err();
    assert!(err.to_string().contains(&format!(
        "core dumped at {}",
        coredump_file.path().display()
    )));
    Ok(())
}

// Running simple wat
#[test]
fn run_wasmtime_simple_wat() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/simple.wat")?;
    run_wasmtime(&[
        "run",
        "--invoke",
        "simple",
        "--disable-cache",
        wasm.path().to_str().unwrap(),
        "4",
    ])?;
    assert_eq!(
        run_wasmtime(&[
            "run",
            "--invoke",
            "get_f32",
            "--disable-cache",
            wasm.path().to_str().unwrap(),
        ])?,
        "100\n"
    );
    assert_eq!(
        run_wasmtime(&[
            "run",
            "--invoke",
            "get_f64",
            "--disable-cache",
            wasm.path().to_str().unwrap(),
        ])?,
        "100\n"
    );
    Ok(())
}

// Running a wat that traps.
#[test]
fn run_wasmtime_unreachable_wat() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/unreachable.wat")?;
    let output =
        run_wasmtime_for_output(&[wasm.path().to_str().unwrap(), "--disable-cache"], None)?;

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
    let wasm = build_wasm("tests/all/cli_tests/hello_wasi_snapshot0.wat")?;
    let stdout = run_wasmtime(&["--disable-cache", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

// Run a simple WASI hello world, snapshot1 edition.
#[test]
fn hello_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/hello_wasi_snapshot1.wat")?;
    let stdout = run_wasmtime(&["--disable-cache", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

#[test]
fn timeout_in_start() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/iloop-start.wat")?;
    let output = run_wasmtime_for_output(
        &[
            "run",
            "--wasm-timeout",
            "1ms",
            "--disable-cache",
            wasm.path().to_str().unwrap(),
        ],
        None,
    )?;
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
    let wasm = build_wasm("tests/all/cli_tests/iloop-invoke.wat")?;
    let output = run_wasmtime_for_output(
        &[
            "run",
            "--wasm-timeout",
            "1ms",
            "--disable-cache",
            wasm.path().to_str().unwrap(),
        ],
        None,
    )?;
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

// Exit with a valid non-zero exit code, snapshot0 edition.
#[test]
fn exit2_wasi_snapshot0() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit2_wasi_snapshot0.wat")?;
    let output =
        run_wasmtime_for_output(&["--disable-cache", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 2);
    Ok(())
}

// Exit with a valid non-zero exit code, snapshot1 edition.
#[test]
fn exit2_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit2_wasi_snapshot1.wat")?;
    let output =
        run_wasmtime_for_output(&["--disable-cache", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 2);
    Ok(())
}

// Exit with a valid non-zero exit code, snapshot0 edition.
#[test]
fn exit125_wasi_snapshot0() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit125_wasi_snapshot0.wat")?;
    let output =
        run_wasmtime_for_output(&["--disable-cache", wasm.path().to_str().unwrap()], None)?;
    if cfg!(windows) {
        assert_eq!(output.status.code().unwrap(), 1);
    } else {
        assert_eq!(output.status.code().unwrap(), 125);
    }
    Ok(())
}

// Exit with a valid non-zero exit code, snapshot1 edition.
#[test]
fn exit125_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit125_wasi_snapshot1.wat")?;
    let output =
        run_wasmtime_for_output(&["--disable-cache", wasm.path().to_str().unwrap()], None)?;
    if cfg!(windows) {
        assert_eq!(output.status.code().unwrap(), 1);
    } else {
        assert_eq!(output.status.code().unwrap(), 125);
    }
    Ok(())
}

// Exit with an invalid non-zero exit code, snapshot0 edition.
#[test]
fn exit126_wasi_snapshot0() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit126_wasi_snapshot0.wat")?;
    let output =
        run_wasmtime_for_output(&["--disable-cache", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 1);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid exit status"));
    Ok(())
}

// Exit with an invalid non-zero exit code, snapshot1 edition.
#[test]
fn exit126_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit126_wasi_snapshot1.wat")?;
    let output =
        run_wasmtime_for_output(&[wasm.path().to_str().unwrap(), "--disable-cache"], None)?;
    assert_eq!(output.status.code().unwrap(), 1);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid exit status"));
    Ok(())
}

// Run a minimal command program.
#[test]
fn minimal_command() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/minimal-command.wat")?;
    let stdout = run_wasmtime(&["--disable-cache", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "");
    Ok(())
}

// Run a minimal reactor program.
#[test]
fn minimal_reactor() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/minimal-reactor.wat")?;
    let stdout = run_wasmtime(&["--disable-cache", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "");
    Ok(())
}

// Attempt to call invoke on a command.
#[test]
fn command_invoke() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/minimal-command.wat")?;
    run_wasmtime(&[
        "run",
        "--invoke",
        "_start",
        "--disable-cache",
        wasm.path().to_str().unwrap(),
    ])?;
    Ok(())
}

// Attempt to call invoke on a command.
#[test]
fn reactor_invoke() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/minimal-reactor.wat")?;
    run_wasmtime(&[
        "run",
        "--invoke",
        "_initialize",
        "--disable-cache",
        wasm.path().to_str().unwrap(),
    ])?;
    Ok(())
}

// Run the greeter test, which runs a preloaded reactor and a command.
#[test]
fn greeter() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/greeter_command.wat")?;
    let stdout = run_wasmtime(&[
        "run",
        "--disable-cache",
        "--preload",
        "reactor=tests/all/cli_tests/greeter_reactor.wat",
        wasm.path().to_str().unwrap(),
    ])?;
    assert_eq!(
        stdout,
        "Hello _initialize\nHello _start\nHello greet\nHello done\n"
    );
    Ok(())
}

// Run the greeter test, but this time preload a command.
#[test]
fn greeter_preload_command() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/greeter_reactor.wat")?;
    let stdout = run_wasmtime(&[
        "run",
        "--disable-cache",
        "--preload",
        "reactor=tests/all/cli_tests/hello_wasi_snapshot1.wat",
        wasm.path().to_str().unwrap(),
    ])?;
    assert_eq!(stdout, "Hello _initialize\n");
    Ok(())
}

// Run the greeter test, which runs a preloaded reactor and a command.
#[test]
fn greeter_preload_callable_command() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/greeter_command.wat")?;
    let stdout = run_wasmtime(&[
        "run",
        "--disable-cache",
        "--preload",
        "reactor=tests/all/cli_tests/greeter_callable_command.wat",
        wasm.path().to_str().unwrap(),
    ])?;
    assert_eq!(stdout, "Hello _start\nHello callable greet\nHello done\n");
    Ok(())
}

// Ensure successful WASI exit call with FPR saving frames on stack for Windows x64
// See https://github.com/bytecodealliance/wasmtime/issues/1967
#[test]
fn exit_with_saved_fprs() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit_with_saved_fprs.wat")?;
    let output =
        run_wasmtime_for_output(&["--disable-cache", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 0);
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn run_cwasm() -> Result<()> {
    let td = TempDir::new()?;
    let cwasm = td.path().join("foo.cwasm");
    let stdout = run_wasmtime(&[
        "compile",
        "tests/all/cli_tests/simple.wat",
        "-o",
        cwasm.to_str().unwrap(),
    ])?;
    assert_eq!(stdout, "");
    let stdout = run_wasmtime(&["run", "--allow-precompiled", cwasm.to_str().unwrap()])?;
    assert_eq!(stdout, "");
    Ok(())
}

#[cfg(unix)]
#[test]
fn hello_wasi_snapshot0_from_stdin() -> Result<()> {
    // Run a simple WASI hello world, snapshot0 edition.
    // The module is piped from standard input.
    let wasm = build_wasm("tests/all/cli_tests/hello_wasi_snapshot0.wat")?;
    let stdout = {
        let path = wasm.path();
        let args: &[&str] = &["--disable-cache", "-"];
        let output = run_wasmtime_for_output(args, Some(path))?;
        if !output.status.success() {
            bail!(
                "Failed to execute wasmtime with: {:?}\n{}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok::<_, anyhow::Error>(String::from_utf8(output.stdout).unwrap())
    }?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

#[test]
fn specify_env() -> Result<()> {
    // By default no env is inherited
    let output = get_wasmtime_command()?
        .args(&["run", "tests/all/cli_tests/print_env.wat"])
        .env("THIS_WILL_NOT", "show up in the output")
        .output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");

    // Specify a single env var
    let output = get_wasmtime_command()?
        .args(&[
            "run",
            "--env",
            "FOO=bar",
            "tests/all/cli_tests/print_env.wat",
        ])
        .output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "FOO=bar\n");

    // Inherit a single env var
    let output = get_wasmtime_command()?
        .args(&["run", "--env", "FOO", "tests/all/cli_tests/print_env.wat"])
        .env("FOO", "bar")
        .output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "FOO=bar\n");

    // Inherit a nonexistent env var
    let output = get_wasmtime_command()?
        .args(&[
            "run",
            "--env",
            "SURELY_THIS_ENV_VAR_DOES_NOT_EXIST_ANYWHERE_RIGHT",
            "tests/all/cli_tests/print_env.wat",
        ])
        .output()?;
    assert!(!output.status.success());

    Ok(())
}

#[cfg(unix)]
#[test]
fn run_cwasm_from_stdin() -> Result<()> {
    use std::process::Stdio;

    let td = TempDir::new()?;
    let cwasm = td.path().join("foo.cwasm");
    let stdout = run_wasmtime(&[
        "compile",
        "tests/all/cli_tests/simple.wat",
        "-o",
        cwasm.to_str().unwrap(),
    ])?;
    assert_eq!(stdout, "");

    // If stdin is literally the file itself then that should work
    let args: &[&str] = &["run", "--allow-precompiled", "-"];
    let output = get_wasmtime_command()?
        .args(args)
        .stdin(File::open(&cwasm)?)
        .output()?;
    assert!(output.status.success(), "a file as stdin should work");

    // If stdin is a pipe, however, that should fail
    let input = std::fs::read(&cwasm)?;
    let mut child = get_wasmtime_command()?
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    let t = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let output = child.wait_with_output()?;
    if output.status.success() {
        bail!("wasmtime should fail loading precompiled modules from piped files, but suceeded");
    }
    t.join().unwrap();
    Ok(())
}

#[cfg(feature = "wasi-threads")]
#[test]
fn run_threads() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/threads.wat")?;
    let stdout = run_wasmtime(&[
        "run",
        "--wasi-modules",
        "experimental-wasi-threads",
        "--wasm-features",
        "threads",
        "--disable-cache",
        wasm.path().to_str().unwrap(),
    ])?;

    assert!(
        stdout
            == "Called _start\n\
    Running wasi_thread_start\n\
    Running wasi_thread_start\n\
    Running wasi_thread_start\n\
    Done\n"
    );
    Ok(())
}

#[cfg(feature = "wasi-threads")]
#[test]
fn run_simple_with_wasi_threads() -> Result<()> {
    // We expect to be able to run Wasm modules that do not have correct
    // wasi-thread entry points or imported shared memory as long as no threads
    // are spawned.
    let wasm = build_wasm("tests/all/cli_tests/simple.wat")?;
    let stdout = run_wasmtime(&[
        "run",
        "--wasi-modules",
        "experimental-wasi-threads",
        "--wasm-features",
        "threads",
        "--disable-cache",
        "--invoke",
        "simple",
        wasm.path().to_str().unwrap(),
        "4",
    ])?;
    assert_eq!(stdout, "4\n");
    Ok(())
}

#[test]
fn wasm_flags() -> Result<()> {
    // Any argument after the wasm module should be interpreted as for the
    // command itself
    let stdout = run_wasmtime(&[
        "run",
        "tests/all/cli_tests/print-arguments.wat",
        "--argument",
        "-for",
        "the",
        "command",
    ])?;
    assert_eq!(
        stdout,
        "\
            print-arguments.wat\n\
            --argument\n\
            -for\n\
            the\n\
            command\n\
        "
    );
    let stdout = run_wasmtime(&["run", "tests/all/cli_tests/print-arguments.wat", "-"])?;
    assert_eq!(
        stdout,
        "\
            print-arguments.wat\n\
            -\n\
        "
    );
    let stdout = run_wasmtime(&["run", "tests/all/cli_tests/print-arguments.wat", "--"])?;
    assert_eq!(
        stdout,
        "\
            print-arguments.wat\n\
            --\n\
        "
    );
    let stdout = run_wasmtime(&[
        "run",
        "tests/all/cli_tests/print-arguments.wat",
        "--",
        "--",
        "-a",
        "b",
    ])?;
    assert_eq!(
        stdout,
        "\
            print-arguments.wat\n\
            --\n\
            --\n\
            -a\n\
            b\n\
        "
    );
    Ok(())
}

#[test]
fn name_same_as_builtin_command() -> Result<()> {
    // a bare subcommand shouldn't run successfully
    let output = get_wasmtime_command()?
        .current_dir("tests/all/cli_tests")
        .arg("run")
        .output()?;
    assert!(!output.status.success());

    // a `--` prefix should let everything else get interpreted as a wasm
    // module and arguments, even if the module has a name like `run`
    let output = get_wasmtime_command()?
        .current_dir("tests/all/cli_tests")
        .arg("--")
        .arg("run")
        .output()?;
    assert!(output.status.success(), "expected success got {output:#?}");

    // Passing options before the subcommand should work and doesn't require
    // `--` to disambiguate
    let output = get_wasmtime_command()?
        .current_dir("tests/all/cli_tests")
        .arg("--disable-cache")
        .arg("run")
        .output()?;
    assert!(output.status.success(), "expected success got {output:#?}");
    Ok(())
}

#[test]
#[cfg(unix)]
fn run_just_stdin_argument() -> Result<()> {
    let output = get_wasmtime_command()?
        .arg("-")
        .stdin(File::open("tests/all/cli_tests/simple.wat")?)
        .output()?;
    assert!(output.status.success());
    Ok(())
}

#[test]
fn wasm_flags_without_subcommand() -> Result<()> {
    let output = get_wasmtime_command()?
        .current_dir("tests/all/cli_tests/")
        .arg("print-arguments.wat")
        .arg("-foo")
        .arg("bar")
        .output()?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "\
            print-arguments.wat\n\
            -foo\n\
            bar\n\
        "
    );
    Ok(())
}

#[test]
fn wasi_misaligned_pointer() -> Result<()> {
    let output = get_wasmtime_command()?
        .arg("./tests/all/cli_tests/wasi_misaligned_pointer.wat")
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Pointer not aligned"),
        "bad stderr: {stderr}",
    );
    Ok(())
}
