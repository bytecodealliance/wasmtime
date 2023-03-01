use anyhow::{Context, Result};
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::{self, DirEntry, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[test]
fn testsuite() -> Result<()> {
    for module in list_modules("spec/test/testsuite")? {
        println!("Testing {}", module.display());
        let result = run(&module)?;
        let spec = parse_spec(&module.with_extension("json"))?;
        assert_eq!(spec, result);
    }
    Ok(())
}

fn run<P: AsRef<Path>>(module: P) -> Result<Output> {
    run_wasmtime_for_output(
        &[
            "run",
            "--wasi-modules",
            "experimental-wasi-threads",
            "--wasm-features",
            "threads",
            "--disable-cache",
            module.as_ref().to_str().unwrap(),
        ],
        None,
    )
}

fn list_modules(testsuite_dir: &str) -> Result<impl Iterator<Item = PathBuf>> {
    Ok(fs::read_dir(testsuite_dir)?
        .filter_map(Result::ok)
        .filter(is_wasm)
        .map(|e| e.path()))
}

fn is_wasm(entry: &DirEntry) -> bool {
    let path = entry.path();
    let ext = path.extension().map(OsStr::to_str).flatten();
    path.is_file() && (ext == Some("wat") || ext == Some("wasm"))
}

fn parse_spec<P: AsRef<Path>>(spec_file: P) -> Result<Spec> {
    let contents = fs::read_to_string(spec_file)?;
    let spec = serde_json::from_str(&contents)?;
    Ok(spec)
}

#[derive(Debug, Deserialize)]
struct Spec {
    exit_code: i32,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl PartialEq<Output> for Spec {
    fn eq(&self, other: &Output) -> bool {
        self.exit_code == other.status.code().unwrap()
            && matches_or_missing(&self.stdout, &other.stdout)
            && matches_or_missing(&self.stderr, &other.stderr)
    }
}

fn matches_or_missing(a: &Option<String>, b: &[u8]) -> bool {
    a.as_ref()
        .map(|s| s == &String::from_utf8_lossy(b))
        .unwrap_or(true)
}

// Run the wasmtime CLI with the provided args and return the `Output`.
// If the `stdin` is `Some`, opens the file and redirects to the child's stdin.
fn run_wasmtime_for_output(args: &[&str], stdin: Option<&Path>) -> Result<Output> {
    let runner = std::env::vars()
        .filter(|(k, _v)| k.starts_with("CARGO_TARGET") && k.ends_with("RUNNER"))
        .next();
    let mut me = std::env::current_exe()?;
    me.pop(); // chop off the file name
    me.pop(); // chop off `deps`
    me.push("wasmtime");

    let stdin = stdin
        .map(File::open)
        .transpose()
        .context("Cannot open a file to use as stdin")?;

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

    if let Some(mut f) = stdin {
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        let mut child = cmd
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .args(args)
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            stdin
                .write_all(&buf)
                .expect("failed to write module to child stdin")
        });
        child.wait_with_output().map_err(Into::into)
    } else {
        cmd.args(args).output().map_err(Into::into)
    }
}
