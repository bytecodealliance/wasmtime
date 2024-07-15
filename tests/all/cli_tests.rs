#![cfg(not(miri))]

use anyhow::{bail, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
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
    let mut cmd = if let Some((_, runner)) = runner {
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

    // Ignore this if it's specified in the environment to allow tests to run in
    // "default mode" by default.
    cmd.env_remove("WASMTIME_NEW_CLI");

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
        "-Ccache=n",
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
            "-Ccache=n",
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
    let coredump_arg = format!("-Dcoredump={}", coredump_file.path().display());
    let err = run_wasmtime(&[
        "run",
        "--invoke",
        "a",
        "-Ccache=n",
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
        "-Ccache=n",
        wasm.path().to_str().unwrap(),
        "4",
    ])?;
    assert_eq!(
        run_wasmtime(&[
            "run",
            "--invoke",
            "get_f32",
            "-Ccache=n",
            wasm.path().to_str().unwrap(),
        ])?,
        "100\n"
    );
    assert_eq!(
        run_wasmtime(&[
            "run",
            "--invoke",
            "get_f64",
            "-Ccache=n",
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
    let output = run_wasmtime_for_output(&[wasm.path().to_str().unwrap(), "-Ccache=n"], None)?;

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
    for preview2 in ["-Spreview2=n", "-Spreview2=y"] {
        let stdout = run_wasmtime(&["-Ccache=n", preview2, wasm.path().to_str().unwrap()])?;
        assert_eq!(stdout, "Hello, world!\n");
    }
    Ok(())
}

// Run a simple WASI hello world, snapshot1 edition.
#[test]
fn hello_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/hello_wasi_snapshot1.wat")?;
    let stdout = run_wasmtime(&["-Ccache=n", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

#[test]
fn timeout_in_start() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/iloop-start.wat")?;
    let output = run_wasmtime_for_output(
        &[
            "run",
            "-Wtimeout=1ms",
            "-Ccache=n",
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
            "-Wtimeout=1ms",
            "-Ccache=n",
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

    for preview2 in ["-Spreview2=n", "-Spreview2=y"] {
        let output = run_wasmtime_for_output(
            &["-Ccache=n", preview2, wasm.path().to_str().unwrap()],
            None,
        )?;
        assert_eq!(output.status.code().unwrap(), 2);
    }
    Ok(())
}

// Exit with a valid non-zero exit code, snapshot1 edition.
#[test]
fn exit2_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit2_wasi_snapshot1.wat")?;
    let output = run_wasmtime_for_output(&["-Ccache=n", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 2);
    Ok(())
}

// Exit with a valid non-zero exit code, snapshot0 edition.
#[test]
fn exit125_wasi_snapshot0() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit125_wasi_snapshot0.wat")?;
    for preview2 in ["-Spreview2=n", "-Spreview2=y"] {
        let output = run_wasmtime_for_output(
            &["-Ccache=n", preview2, wasm.path().to_str().unwrap()],
            None,
        )?;
        dbg!(&output);
        if cfg!(windows) {
            assert_eq!(output.status.code().unwrap(), 1);
        } else {
            assert_eq!(output.status.code().unwrap(), 125);
        }
    }
    Ok(())
}

// Exit with a valid non-zero exit code, snapshot1 edition.
#[test]
fn exit125_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit125_wasi_snapshot1.wat")?;
    let output = run_wasmtime_for_output(&["-Ccache=n", wasm.path().to_str().unwrap()], None)?;
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

    for preview2 in ["-Spreview2=n", "-Spreview2=y"] {
        let output = run_wasmtime_for_output(
            &["-Ccache=n", preview2, wasm.path().to_str().unwrap()],
            None,
        )?;
        assert_eq!(output.status.code().unwrap(), 1);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid exit status"));
    }
    Ok(())
}

// Exit with an invalid non-zero exit code, snapshot1 edition.
#[test]
fn exit126_wasi_snapshot1() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/exit126_wasi_snapshot1.wat")?;
    let output = run_wasmtime_for_output(&[wasm.path().to_str().unwrap(), "-Ccache=n"], None)?;
    assert_eq!(output.status.code().unwrap(), 1);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid exit status"));
    Ok(())
}

// Run a minimal command program.
#[test]
fn minimal_command() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/minimal-command.wat")?;
    let stdout = run_wasmtime(&["-Ccache=n", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "");
    Ok(())
}

// Run a minimal reactor program.
#[test]
fn minimal_reactor() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/minimal-reactor.wat")?;
    let stdout = run_wasmtime(&["-Ccache=n", wasm.path().to_str().unwrap()])?;
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
        "-Ccache=n",
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
        "-Ccache=n",
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
        "-Ccache=n",
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
        "-Ccache=n",
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
        "-Ccache=n",
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
    let output = run_wasmtime_for_output(&["-Ccache=n", wasm.path().to_str().unwrap()], None)?;
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
    for preview2 in ["-Spreview2=n", "-Spreview2=y"] {
        let stdout = {
            let path = wasm.path();
            let args: &[&str] = &["-Ccache=n", preview2, "-"];
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
    }
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
    assert!(output.status.success());

    // Inherit all env vars
    let output = get_wasmtime_command()?
        .args(&["run", "-Sinherit-env", "tests/all/cli_tests/print_env.wat"])
        .env("FOO", "bar")
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FOO=bar"), "bad output: {stdout}");

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

    // If stdin is a pipe, that should also work
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
    assert!(output.status.success());
    t.join().unwrap();
    Ok(())
}

#[cfg(feature = "wasi-threads")]
#[test]
fn run_threads() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/threads.wat")?;
    let stdout = run_wasmtime(&[
        "run",
        "-Wthreads",
        "-Sthreads",
        "-Ccache=n",
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
        "-Wthreads",
        "-Sthreads",
        "-Ccache=n",
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
        "--",
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
    let stdout = run_wasmtime(&["run", "--", "tests/all/cli_tests/print-arguments.wat", "-"])?;
    assert_eq!(
        stdout,
        "\
            print-arguments.wat\n\
            -\n\
        "
    );
    let stdout = run_wasmtime(&["run", "--", "tests/all/cli_tests/print-arguments.wat", "--"])?;
    assert_eq!(
        stdout,
        "\
            print-arguments.wat\n\
            --\n\
        "
    );
    let stdout = run_wasmtime(&[
        "run",
        "--",
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
        .arg("-Ccache=n")
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

#[test]
#[cfg_attr(not(feature = "component-model"), ignore)]
fn hello_with_preview2() -> Result<()> {
    let wasm = build_wasm("tests/all/cli_tests/hello_wasi_snapshot1.wat")?;
    let stdout = run_wasmtime(&["-Ccache=n", "-Spreview2", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "Hello, world!\n");
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "component-model"), ignore)]
fn component_missing_feature() -> Result<()> {
    let path = "tests/all/cli_tests/empty-component.wat";
    let wasm = build_wasm(path)?;
    let output = get_wasmtime_command()?
        .arg("-Ccache=n")
        .arg("-Wcomponent-model=n")
        .arg(wasm.path())
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot execute a component without `--wasm component-model`"),
        "bad stderr: {stderr}"
    );

    // also tests with raw *.wat input
    let output = get_wasmtime_command()?
        .arg("-Ccache=n")
        .arg("-Wcomponent-model=n")
        .arg(path)
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot execute a component without `--wasm component-model`"),
        "bad stderr: {stderr}"
    );

    Ok(())
}

#[test]
#[cfg_attr(not(feature = "component-model"), ignore)]
fn component_enabled_by_default() -> Result<()> {
    let path = "tests/all/cli_tests/component-basic.wat";
    let wasm = build_wasm(path)?;
    let output = get_wasmtime_command()?
        .arg("-Ccache=n")
        .arg(wasm.path())
        .output()?;
    assert!(output.status.success());

    // also tests with raw *.wat input
    let output = get_wasmtime_command()?
        .arg("-Ccache=n")
        .arg(path)
        .output()?;
    assert!(output.status.success());

    Ok(())
}

// If the text format is invalid then the filename should be mentioned in the
// error message.
#[test]
fn bad_text_syntax() -> Result<()> {
    let output = get_wasmtime_command()?
        .arg("-Ccache=n")
        .arg("tests/all/cli_tests/bad-syntax.wat")
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--> tests/all/cli_tests/bad-syntax.wat"),
        "bad stderr: {stderr}"
    );
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "component-model"), ignore)]
fn run_basic_component() -> Result<()> {
    let path = "tests/all/cli_tests/component-basic.wat";
    let wasm = build_wasm(path)?;

    // Run both the `*.wasm` binary and the text format
    run_wasmtime(&[
        "-Ccache=n",
        "-Wcomponent-model",
        wasm.path().to_str().unwrap(),
    ])?;
    run_wasmtime(&["-Ccache=n", "-Wcomponent-model", path])?;

    Ok(())
}

#[test]
#[cfg_attr(not(feature = "component-model"), ignore)]
fn run_precompiled_component() -> Result<()> {
    let td = TempDir::new()?;
    let cwasm = td.path().join("component-basic.cwasm");
    let stdout = run_wasmtime(&[
        "compile",
        "tests/all/cli_tests/component-basic.wat",
        "-o",
        cwasm.to_str().unwrap(),
        "-Wcomponent-model",
    ])?;
    assert_eq!(stdout, "");
    let stdout = run_wasmtime(&[
        "run",
        "-Wcomponent-model",
        "--allow-precompiled",
        cwasm.to_str().unwrap(),
    ])?;
    assert_eq!(stdout, "");

    Ok(())
}

#[test]
fn memory_growth_failure() -> Result<()> {
    let output = get_wasmtime_command()?
        .args(&[
            "run",
            "-Wmemory64",
            "-Wtrap-on-grow-failure",
            "tests/all/cli_tests/memory-grow-failure.wat",
        ])
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("forcing a memory growth failure to be a trap"),
        "bad stderr: {stderr}"
    );
    Ok(())
}

#[test]
fn table_growth_failure() -> Result<()> {
    let output = get_wasmtime_command()?
        .args(&[
            "run",
            "-Wtrap-on-grow-failure",
            "tests/all/cli_tests/table-grow-failure.wat",
        ])
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("forcing trap when growing table"),
        "bad stderr: {stderr}"
    );
    Ok(())
}

#[test]
fn table_growth_failure2() -> Result<()> {
    let output = get_wasmtime_command()?
        .args(&[
            "run",
            "-Wtrap-on-grow-failure",
            "tests/all/cli_tests/table-grow-failure2.wat",
        ])
        .output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("forcing a table growth failure to be a trap"),
        "bad stderr: {stderr}"
    );
    Ok(())
}

#[test]
fn option_group_help() -> Result<()> {
    run_wasmtime(&["run", "-Whelp"])?;
    run_wasmtime(&["run", "-O", "help"])?;
    run_wasmtime(&["run", "--codegen", "help"])?;
    run_wasmtime(&["run", "--debug=help"])?;
    run_wasmtime(&["run", "-Shelp"])?;
    run_wasmtime(&["run", "-Whelp-long"])?;
    Ok(())
}

#[test]
fn option_group_comma_separated() -> Result<()> {
    run_wasmtime(&[
        "run",
        "-Wrelaxed-simd,simd",
        "tests/all/cli_tests/simple.wat",
    ])?;
    Ok(())
}

#[test]
fn option_group_boolean_parsing() -> Result<()> {
    run_wasmtime(&["run", "-Wrelaxed-simd", "tests/all/cli_tests/simple.wat"])?;
    run_wasmtime(&["run", "-Wrelaxed-simd=n", "tests/all/cli_tests/simple.wat"])?;
    run_wasmtime(&["run", "-Wrelaxed-simd=y", "tests/all/cli_tests/simple.wat"])?;
    run_wasmtime(&["run", "-Wrelaxed-simd=no", "tests/all/cli_tests/simple.wat"])?;
    run_wasmtime(&[
        "run",
        "-Wrelaxed-simd=yes",
        "tests/all/cli_tests/simple.wat",
    ])?;
    run_wasmtime(&[
        "run",
        "-Wrelaxed-simd=true",
        "tests/all/cli_tests/simple.wat",
    ])?;
    run_wasmtime(&[
        "run",
        "-Wrelaxed-simd=false",
        "tests/all/cli_tests/simple.wat",
    ])?;
    Ok(())
}

#[test]
fn preview2_stdin() -> Result<()> {
    let test = "tests/all/cli_tests/count-stdin.wat";
    let cmd = || -> Result<_> {
        let mut cmd = get_wasmtime_command()?;
        cmd.arg("--invoke=count").arg("-Spreview2").arg(test);
        Ok(cmd)
    };

    // read empty pipe is ok
    let output = cmd()?.output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "0\n");

    // read itself is ok
    let file = File::open(test)?;
    let size = file.metadata()?.len();
    let output = cmd()?.stdin(File::open(test)?).output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("{size}\n"));

    // read piped input ok is ok
    let mut child = cmd()?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    std::thread::spawn(move || {
        stdin.write_all(b"hello").unwrap();
    });
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "5\n");

    let count_up_to = |n: usize| -> Result<_> {
        let mut child = get_wasmtime_command()?
            .arg("--invoke=count-up-to")
            .arg("-Spreview2")
            .arg(test)
            .arg(n.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().unwrap();
        let t = std::thread::spawn(move || {
            let mut written = 0;
            let bytes = [0; 64 * 1024];
            loop {
                written += match stdin.write(&bytes) {
                    Ok(n) => n,
                    Err(_) => break written,
                };
            }
        });
        let output = child.wait_with_output()?;
        assert!(output.status.success());
        let written = t.join().unwrap();
        let read = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()
            .unwrap();
        // The test reads in 1000 byte chunks so make sure that it doesn't read
        // more than 1000 bytes than requested.
        assert!(read < n + 1000, "test read too much {read}");
        Ok(written)
    };

    // wasmtime shouldn't eat information that the guest never actually tried to
    // read.
    //
    // NB: this may be a bit flaky. Exactly how much we wrote in the above
    // helper thread depends on how much the OS buffers for us. For now give
    // some some slop and assume that OSes are unlikely to buffer more than
    // that.
    let slop = 256 * 1024;
    for amt in [0, 100, 100_000] {
        let written = count_up_to(amt)?;
        assert!(written < slop + amt, "wrote too much {written}");
    }
    Ok(())
}

#[test]
fn float_args() -> Result<()> {
    let result = run_wasmtime(&[
        "--invoke",
        "echo_f32",
        "tests/all/cli_tests/simple.wat",
        "1.0",
    ])?;
    assert_eq!(result, "1\n");
    let result = run_wasmtime(&[
        "--invoke",
        "echo_f64",
        "tests/all/cli_tests/simple.wat",
        "1.1",
    ])?;
    assert_eq!(result, "1.1\n");
    Ok(())
}

#[test]
fn mpk_without_pooling() -> Result<()> {
    let output = get_wasmtime_command()?
        .args(&[
            "run",
            "-O",
            "memory-protection-keys=y",
            "--invoke",
            "echo_f32",
            "tests/all/cli_tests/simple.wat",
            "1.0",
        ])
        .env("WASMTIME_NEW_CLI", "1")
        .output()?;
    assert!(!output.status.success());
    Ok(())
}

mod test_programs {
    use super::{get_wasmtime_command, run_wasmtime};
    use anyhow::{bail, Context, Result};
    use http_body_util::BodyExt;
    use hyper::header::HeaderValue;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::SocketAddr;
    use std::process::{Child, Command, Stdio};
    use test_programs_artifacts::*;
    use tokio::net::TcpStream;

    macro_rules! assert_test_exists {
        ($name:ident) => {
            #[allow(unused_imports)]
            use self::$name as _;
        };
    }
    foreach_cli!(assert_test_exists);

    #[test]
    fn cli_hello_stdout() -> Result<()> {
        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            CLI_HELLO_STDOUT_COMPONENT,
            "gussie",
            "sparky",
            "willa",
        ])?;
        Ok(())
    }

    #[test]
    fn cli_args() -> Result<()> {
        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            CLI_ARGS_COMPONENT,
            "hello",
            "this",
            "",
            "is an argument",
            "with 🚩 emoji",
        ])?;
        Ok(())
    }

    #[test]
    fn cli_stdin() -> Result<()> {
        let mut child = get_wasmtime_command()?
            .args(&["run", "-Wcomponent-model", CLI_STDIN_COMPONENT])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"So rested he by the Tumtum tree")
            .unwrap();
        let output = child.wait_with_output()?;
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
        Ok(())
    }

    #[test]
    fn cli_splice_stdin() -> Result<()> {
        let mut child = get_wasmtime_command()?
            .args(&["run", "-Wcomponent-model", CLI_SPLICE_STDIN_COMPONENT])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;
        let msg = "So rested he by the Tumtum tree";
        child
            .stdin
            .take()
            .unwrap()
            .write_all(msg.as_bytes())
            .unwrap();
        let output = child.wait_with_output()?;
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            eprintln!("{stderr}");
        }

        assert_eq!(
            format!(
                "before splice\n{msg}\ncompleted splicing {} bytes\n",
                msg.as_bytes().len()
            ),
            stdout
        );
        Ok(())
    }

    #[test]
    fn cli_env() -> Result<()> {
        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            "--env=frabjous=day",
            "--env=callooh=callay",
            CLI_ENV_COMPONENT,
        ])?;
        Ok(())
    }

    #[test]
    fn cli_file_read() -> Result<()> {
        let dir = tempfile::tempdir()?;

        std::fs::write(dir.path().join("bar.txt"), b"And stood awhile in thought")?;

        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            &format!("--dir={}::/", dir.path().to_str().unwrap()),
            CLI_FILE_READ_COMPONENT,
        ])?;
        Ok(())
    }

    #[test]
    fn cli_file_append() -> Result<()> {
        let dir = tempfile::tempdir()?;

        std::fs::File::create(dir.path().join("bar.txt"))?
            .write_all(b"'Twas brillig, and the slithy toves.\n")?;

        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            &format!("--dir={}::/", dir.path().to_str().unwrap()),
            CLI_FILE_APPEND_COMPONENT,
        ])?;

        let contents = std::fs::read(dir.path().join("bar.txt"))?;
        assert_eq!(
            std::str::from_utf8(&contents).unwrap(),
            "'Twas brillig, and the slithy toves.\n\
                   Did gyre and gimble in the wabe;\n\
                   All mimsy were the borogoves,\n\
                   And the mome raths outgrabe.\n"
        );
        Ok(())
    }

    #[test]
    fn cli_file_dir_sync() -> Result<()> {
        let dir = tempfile::tempdir()?;

        std::fs::File::create(dir.path().join("bar.txt"))?
            .write_all(b"'Twas brillig, and the slithy toves.\n")?;

        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            &format!("--dir={}::/", dir.path().to_str().unwrap()),
            CLI_FILE_DIR_SYNC_COMPONENT,
        ])?;

        Ok(())
    }

    #[test]
    fn cli_exit_success() -> Result<()> {
        run_wasmtime(&["run", "-Wcomponent-model", CLI_EXIT_SUCCESS_COMPONENT])?;
        Ok(())
    }

    #[test]
    fn cli_exit_default() -> Result<()> {
        run_wasmtime(&["run", "-Wcomponent-model", CLI_EXIT_DEFAULT_COMPONENT])?;
        Ok(())
    }

    #[test]
    fn cli_exit_failure() -> Result<()> {
        let output = get_wasmtime_command()?
            .args(&["run", "-Wcomponent-model", CLI_EXIT_FAILURE_COMPONENT])
            .output()?;
        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));
        Ok(())
    }

    #[test]
    fn cli_exit_panic() -> Result<()> {
        let output = get_wasmtime_command()?
            .args(&["run", "-Wcomponent-model", CLI_EXIT_PANIC_COMPONENT])
            .output()?;
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Curiouser and curiouser!"));
        Ok(())
    }

    #[test]
    fn cli_directory_list() -> Result<()> {
        let dir = tempfile::tempdir()?;

        std::fs::File::create(dir.path().join("foo.txt"))?;
        std::fs::File::create(dir.path().join("bar.txt"))?;
        std::fs::File::create(dir.path().join("baz.txt"))?;
        std::fs::create_dir(dir.path().join("sub"))?;
        std::fs::File::create(dir.path().join("sub").join("wow.txt"))?;
        std::fs::File::create(dir.path().join("sub").join("yay.txt"))?;

        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            &format!("--dir={}::/", dir.path().to_str().unwrap()),
            CLI_DIRECTORY_LIST_COMPONENT,
        ])?;
        Ok(())
    }

    #[test]
    fn cli_default_clocks() -> Result<()> {
        run_wasmtime(&["run", "-Wcomponent-model", CLI_DEFAULT_CLOCKS_COMPONENT])?;
        Ok(())
    }

    #[test]
    fn cli_export_cabi_realloc() -> Result<()> {
        run_wasmtime(&[
            "run",
            "-Wcomponent-model",
            CLI_EXPORT_CABI_REALLOC_COMPONENT,
        ])?;
        Ok(())
    }

    #[test]
    fn run_wasi_http_component() -> Result<()> {
        let output = super::run_wasmtime_for_output(
            &[
                "-Ccache=no",
                "-Wcomponent-model",
                "-Scli,http,preview2",
                HTTP_OUTBOUND_REQUEST_RESPONSE_BUILD_COMPONENT,
            ],
            None,
        )?;
        println!("{}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
        assert!(stdout.starts_with("Called _start\n"));
        assert!(stdout.ends_with("Done\n"));
        assert!(output.status.success());
        Ok(())
    }

    // Test to ensure that prints in the guest aren't buffered on the host by
    // accident. The test here will print something without a newline and then
    // wait for input on stdin, and the test here is to ensure that the
    // character shows up here even as the guest is waiting on input via stdin.
    #[test]
    fn cli_stdio_write_flushes() -> Result<()> {
        fn run(args: &[&str]) -> Result<()> {
            println!("running {args:?}");
            let mut child = get_wasmtime_command()?
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            let mut stdout = child.stdout.take().unwrap();
            let mut buf = [0; 10];
            match stdout.read(&mut buf) {
                Ok(2) => assert_eq!(&buf[..2], b"> "),
                e => panic!("unexpected read result {e:?}"),
            }
            drop(stdout);
            drop(child.stdin.take().unwrap());
            let status = child.wait()?;
            assert!(status.success());
            Ok(())
        }

        run(&["run", "-Spreview2=n", CLI_STDIO_WRITE_FLUSHES])?;
        run(&["run", "-Spreview2=y", CLI_STDIO_WRITE_FLUSHES])?;
        run(&[
            "run",
            "-Wcomponent-model",
            CLI_STDIO_WRITE_FLUSHES_COMPONENT,
        ])?;
        Ok(())
    }

    #[test]
    fn cli_no_tcp() -> Result<()> {
        let output = super::run_wasmtime_for_output(
            &[
                "-Wcomponent-model",
                // Turn on network but turn off TCP
                "-Sinherit-network,tcp=no",
                CLI_NO_TCP_COMPONENT,
            ],
            None,
        )?;
        println!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
        Ok(())
    }

    #[test]
    fn cli_no_udp() -> Result<()> {
        let output = super::run_wasmtime_for_output(
            &[
                "-Wcomponent-model",
                // Turn on network but turn off UDP
                "-Sinherit-network,udp=no",
                CLI_NO_UDP_COMPONENT,
            ],
            None,
        )?;
        println!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
        Ok(())
    }

    #[test]
    fn cli_no_ip_name_lookup() -> Result<()> {
        let output = super::run_wasmtime_for_output(
            &[
                "-Wcomponent-model",
                // Turn on network but ensure name lookup is disabled
                "-Sinherit-network,allow-ip-name-lookup=no",
                CLI_NO_IP_NAME_LOOKUP_COMPONENT,
            ],
            None,
        )?;
        println!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
        Ok(())
    }

    #[test]
    fn cli_sleep() -> Result<()> {
        run_wasmtime(&["run", CLI_SLEEP])?;
        run_wasmtime(&["run", CLI_SLEEP_COMPONENT])?;
        Ok(())
    }

    /// Helper structure to manage an invocation of `wasmtime serve`
    struct WasmtimeServe {
        child: Option<Child>,
        addr: SocketAddr,
    }

    impl WasmtimeServe {
        /// Creates a new server which will serve the wasm component pointed to
        /// by `wasm`.
        ///
        /// A `configure` callback is provided to specify how `wasmtime serve`
        /// will be invoked and configure arguments such as headers.
        fn new(wasm: &str, configure: impl FnOnce(&mut Command)) -> Result<WasmtimeServe> {
            // Spawn `wasmtime serve` on port 0 which will randomly assign it a
            // port.
            let mut cmd = super::get_wasmtime_command()?;
            cmd.arg("serve").arg("--addr=127.0.0.1:0").arg(wasm);
            configure(&mut cmd);
            Self::spawn(&mut cmd)
        }

        fn spawn(cmd: &mut Command) -> Result<WasmtimeServe> {
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            let mut child = cmd.spawn()?;

            // Read the first line of stderr which will say which address it's
            // listening on.
            //
            // NB: this intentionally discards any extra buffered data in the
            // `BufReader` once the newline is found. The server shouldn't print
            // anything interesting other than the address so once we get a line
            // all remaining output is left to be captured by future requests
            // send to the server.
            let mut line = String::new();
            let mut reader = BufReader::new(child.stderr.take().unwrap());
            reader.read_line(&mut line)?;

            match line.find("127.0.0.1").and_then(|addr_start| {
                let addr = &line[addr_start..];
                let addr_end = addr.find("/")?;
                addr[..addr_end].parse().ok()
            }) {
                Some(addr) => {
                    assert!(reader.buffer().is_empty());
                    child.stderr = Some(reader.into_inner());
                    Ok(WasmtimeServe {
                        child: Some(child),
                        addr,
                    })
                }
                None => {
                    child.kill()?;
                    child.wait()?;
                    reader.read_to_string(&mut line)?;
                    bail!("failed to start child: {line}")
                }
            }
        }

        /// Completes this server gracefully by printing the output on failure.
        fn finish(mut self) -> Result<(String, String)> {
            let mut child = self.child.take().unwrap();

            // If the child process has already exited then collect the output
            // and test if it succeeded. Otherwise it's still running so kill it
            // and then reap it. Assume that if it's still running then the test
            // has otherwise passed so no need to print the output.
            let known_failure = if child.try_wait()?.is_some() {
                false
            } else {
                child.kill()?;
                true
            };
            let output = child.wait_with_output()?;
            if !known_failure && !output.status.success() {
                bail!("child failed {output:?}");
            }

            Ok((
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }

        /// Send a request to this server and wait for the response.
        async fn send_request(&self, req: http::Request<String>) -> Result<http::Response<String>> {
            let (mut send, conn_task) = self.start_requests().await?;

            let response = send
                .send_request(req)
                .await
                .context("error sending request")?;
            drop(send);
            let (parts, body) = response.into_parts();

            let body = body.collect().await.context("failed to read body")?;
            assert!(body.trailers().is_none());
            let body = std::str::from_utf8(&body.to_bytes())?.to_string();

            conn_task.await??;

            Ok(http::Response::from_parts(parts, body))
        }

        async fn start_requests(
            &self,
        ) -> Result<(
            hyper::client::conn::http1::SendRequest<String>,
            tokio::task::JoinHandle<hyper::Result<()>>,
        )> {
            let tcp = TcpStream::connect(&self.addr)
                .await
                .context("failed to connect")?;
            let tcp = wasmtime_wasi_http::io::TokioIo::new(tcp);
            let (send, conn) = hyper::client::conn::http1::handshake(tcp)
                .await
                .context("failed http handshake")?;
            Ok((send, tokio::task::spawn(conn)))
        }
    }

    // Don't leave child processes running by accident so kill the child process
    // if our server goes away.
    impl Drop for WasmtimeServe {
        fn drop(&mut self) {
            let mut child = match self.child.take() {
                Some(child) => child,
                None => return,
            };
            if child.kill().is_err() {
                return;
            }
            let output = match child.wait_with_output() {
                Ok(output) => output,
                Err(_) => return,
            };

            println!("server status: {}", output.status);
            if !output.stdout.is_empty() {
                println!(
                    "server stdout:\n{}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                println!(
                    "server stderr:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }

    #[tokio::test]
    async fn cli_serve_echo_env() -> Result<()> {
        let server = WasmtimeServe::new(CLI_SERVE_ECHO_ENV_COMPONENT, |cmd| {
            cmd.arg("--env=FOO=bar");
            cmd.arg("--env=BAR");
            cmd.arg("-Scli");
            cmd.env_remove("BAR");
        })?;

        let foo_env = server
            .send_request(
                hyper::Request::builder()
                    .uri("http://localhost/")
                    .header("env", "FOO")
                    .body(String::new())
                    .context("failed to make request")?,
            )
            .await?;

        assert!(foo_env.status().is_success());
        assert!(foo_env.body().is_empty());
        let headers = foo_env.headers();
        assert_eq!(headers.get("env"), Some(&HeaderValue::from_static("bar")));

        let bar_env = server
            .send_request(
                hyper::Request::builder()
                    .uri("http://localhost/")
                    .header("env", "BAR")
                    .body(String::new())
                    .context("failed to make request")?,
            )
            .await?;

        assert!(bar_env.status().is_success());
        assert!(bar_env.body().is_empty());
        let headers = bar_env.headers();
        assert_eq!(headers.get("env"), None);

        server.finish()?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // TODO: printing stderr in the child and killing the child at the
              // end of this test race so the stderr may be present or not. Need
              // to implement a more graceful shutdown routine for `wasmtime
              // serve`.
    async fn cli_serve_respect_pooling_options() -> Result<()> {
        let server = WasmtimeServe::new(CLI_SERVE_ECHO_ENV_COMPONENT, |cmd| {
            cmd.arg("-Opooling-total-memories=0").arg("-Scli");
        })?;

        let result = server
            .send_request(
                hyper::Request::builder()
                    .uri("http://localhost/")
                    .header("env", "FOO")
                    .body(String::new())
                    .context("failed to make request")?,
            )
            .await;
        assert!(result.is_err());
        let (_, stderr) = server.finish()?;
        assert!(
            stderr.contains("maximum concurrent memory limit of 0 reached"),
            "bad stderr: {stderr}",
        );
        Ok(())
    }

    #[test]
    fn cli_large_env() -> Result<()> {
        for wasm in [CLI_LARGE_ENV, CLI_LARGE_ENV_COMPONENT] {
            println!("run {wasm:?}");
            let mut cmd = get_wasmtime_command()?;
            cmd.arg("run").arg("-Sinherit-env").arg(wasm);

            let debug_cmd = format!("{cmd:?}");
            for i in 0..512 {
                let var = format!("KEY{i}");
                let val = (0..1024).map(|_| 'x').collect::<String>();
                cmd.env(&var, &val);
            }
            let output = cmd.output()?;
            if !output.status.success() {
                bail!(
                    "Failed to execute wasmtime with: {debug_cmd}\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn cli_serve_only_one_process_allowed() -> Result<()> {
        let wasm = CLI_SERVE_ECHO_ENV_COMPONENT;
        let server = WasmtimeServe::new(wasm, |cmd| {
            cmd.arg("-Scli");
        })?;

        let err = WasmtimeServe::spawn(
            super::get_wasmtime_command()?
                .arg("serve")
                .arg("-Scli")
                .arg(format!("--addr={}", server.addr))
                .arg(wasm),
        )
        .err()
        .expect("server spawn should have failed but it succeeded");
        drop(server);

        let err = format!("{err:?}");
        println!("{err}");
        assert!(err.contains("os error"));
        Ok(())
    }

    // Technically this test is a little racy. This binds port 0 to acquire a
    // random port, issues a single request to this port, but then kills this
    // server while the request is still processing. The port is then rebound
    // in the next process while it technically could be stolen by another
    // process.
    #[tokio::test]
    async fn cli_serve_quick_rebind_allowed() -> Result<()> {
        let wasm = CLI_SERVE_ECHO_ENV_COMPONENT;
        let server = WasmtimeServe::new(wasm, |cmd| {
            cmd.arg("-Scli");
        })?;
        let addr = server.addr;

        // Start up a `send` and `conn_task` which represents a connection to
        // this server.
        let (mut send, conn_task) = server.start_requests().await?;
        let _ = send
            .send_request(
                hyper::Request::builder()
                    .uri("http://localhost/")
                    .header("env", "FOO")
                    .body(String::new())
                    .context("failed to make request")?,
            )
            .await;

        // ... once a response has been received (or at least the status
        // code/headers) then kill the server. THis is done while `conn_task`
        // and `send` are still alive so we're guaranteed that the other side
        // got a request (we got a response) and our connection is still open.
        //
        // This forces the address/port into the `TIME_WAIT` state. The rebind
        // below in the next process will fail if `SO_REUSEADDR` isn't set.
        drop(server);
        drop(send);
        let _ = conn_task.await;

        // If this is successfully bound then we'll create `WasmtimeServe`
        // which reads off the first line of output to know which address was
        // bound.
        let _server2 = WasmtimeServe::spawn(
            super::get_wasmtime_command()?
                .arg("serve")
                .arg("-Scli")
                .arg(format!("--addr={addr}"))
                .arg(wasm),
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn cli_serve_with_print() -> Result<()> {
        let server = WasmtimeServe::new(CLI_SERVE_WITH_PRINT_COMPONENT, |cmd| {
            cmd.arg("-Scli");
        })?;

        for _ in 0..2 {
            let resp = server
                .send_request(
                    hyper::Request::builder()
                        .uri("http://localhost/")
                        .body(String::new())
                        .context("failed to make request")?,
                )
                .await?;
            assert!(resp.status().is_success());
        }

        let (out, err) = server.finish()?;
        assert_eq!(
            out,
            "\
stdout [0] :: this is half a print to stdout
stdout [0] :: \n\
stdout [0] :: after empty
stdout [1] :: this is half a print to stdout
stdout [1] :: \n\
stdout [1] :: after empty
"
        );
        assert_eq!(
            err,
            "\
stderr [0] :: this is half a print to stderr
stderr [0] :: \n\
stderr [0] :: after empty
stderr [1] :: this is half a print to stderr
stderr [1] :: \n\
stderr [1] :: after empty
"
        );

        Ok(())
    }

    #[tokio::test]
    async fn cli_serve_authority_and_scheme() -> Result<()> {
        let server = WasmtimeServe::new(CLI_SERVE_AUTHORITY_AND_SCHEME_COMPONENT, |cmd| {
            cmd.arg("-Scli");
        })?;

        let resp = server
            .send_request(
                hyper::Request::builder()
                    .uri("/")
                    .header("Host", "localhost")
                    .body(String::new())
                    .context("failed to make request")?,
            )
            .await?;
        assert!(resp.status().is_success());

        let resp = server
            .send_request(
                hyper::Request::builder()
                    .method("CONNECT")
                    .uri("http://localhost/")
                    .body(String::new())
                    .context("failed to make request")?,
            )
            .await?;
        assert!(resp.status().is_success());

        Ok(())
    }

    #[test]
    fn cli_argv0() -> Result<()> {
        run_wasmtime(&["run", "--argv0=a", CLI_ARGV0, "a"])?;
        run_wasmtime(&["run", "--argv0=b", CLI_ARGV0_COMPONENT, "b"])?;
        run_wasmtime(&["run", "--argv0=foo.wasm", CLI_ARGV0, "foo.wasm"])?;
        Ok(())
    }
}

#[test]
fn settings_command() -> Result<()> {
    let output = run_wasmtime(&["settings"])?;
    assert!(output.contains("Cranelift settings for target"));
    Ok(())
}
