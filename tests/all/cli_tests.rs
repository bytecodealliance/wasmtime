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
    let stdout = run_wasmtime(&["-Ccache=n", wasm.path().to_str().unwrap()])?;
    assert_eq!(stdout, "Hello, world!\n");
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
    let output = run_wasmtime_for_output(&["-Ccache=n", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 2);
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
    let output = run_wasmtime_for_output(&["-Ccache=n", wasm.path().to_str().unwrap()], None)?;
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
    let output = run_wasmtime_for_output(&["-Ccache=n", wasm.path().to_str().unwrap()], None)?;
    assert_eq!(output.status.code().unwrap(), 1);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid exit status"));
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
    let stdout = {
        let path = wasm.path();
        let args: &[&str] = &["-Ccache=n", "-"];
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

#[cfg(feature = "wasi-http")]
#[ignore = "needs to be ported to components"]
#[test]
fn run_wasi_http_module() -> Result<()> {
    let output = run_wasmtime_for_output(
        &[
            "-Shttp,preview2",
            "-Ccache=no",
            "tests/all/cli_tests/wasi-http.wat",
        ],
        None,
    )?;
    println!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .starts_with("Called _start\n"));
    assert!(!output.status.success());
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
