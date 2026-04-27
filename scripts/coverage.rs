#!/usr/bin/env -S cargo +nightly -Zscript -q --

//! Generate an LLVM source-based coverage report for `cargo test`.
//!
//! Usage: ./scripts/coverage.rs [cargo-test-args...]
//!
//! Runs `cargo test` with coverage instrumentation enabled, merges the
//! resulting `.profraw` files, and produces an HTML coverage report at
//! `report/index.html`.
//!
//! All arguments are forwarded to `cargo test`. For example, this runs only the
//! tests under `tests/all/*` and the `.wast` tests whose names contain "gc":
//!
//!     ./scripts/coverage.rs -p wasmtime-cli --test all --test wast gc

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let root = find_repo_root();
    let args: Vec<_> = env::args().skip(1).collect();

    let profraw_dir = root.join("target/coverage-profraw");
    let profdata_file = root.join("target/coverage.profdata");
    let report_dir = root.join("report");

    if profraw_dir.exists() {
        fs::remove_dir_all(&profraw_dir).expect("failed to clean profraw dir");
    }
    fs::create_dir_all(&profraw_dir).expect("failed to create profraw dir");

    let llvm_profdata = find_llvm_tool("llvm-profdata");
    let llvm_cov = find_llvm_tool("llvm-cov");

    let mut rustflags = env::var("RUSTFLAGS").unwrap_or_default();
    if !rustflags.is_empty() {
        rustflags.push(' ');
    }
    rustflags.push_str("-C instrument-coverage");

    let profraw_pattern = profraw_dir.join("%m_%p.profraw");

    run_tests(&root, &args, &rustflags, &profraw_pattern);
    let binaries = discover_test_binaries(&root, &args, &rustflags, &profraw_pattern);
    merge_profraw(&llvm_profdata, &profraw_dir, &profdata_file);
    generate_report(&llvm_cov, &binaries, &profdata_file, &report_dir);
}

fn run_tests(root: &Path, args: &[String], rustflags: &str, profraw_pattern: &Path) {
    eprintln!("=== Running cargo test with coverage ===");
    let status = Command::new("cargo")
        .arg("test")
        .args(args)
        .env("RUSTFLAGS", rustflags)
        .env("LLVM_PROFILE_FILE", profraw_pattern)
        .current_dir(root)
        .status()
        .expect("failed to run cargo test");
    if !status.success() {
        eprintln!("cargo test failed with {status}");
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn discover_test_binaries(
    root: &Path,
    args: &[String],
    rustflags: &str,
    profraw_pattern: &Path,
) -> Vec<String> {
    // We need `--no-run --message-format=json` to be cargo flags, not test
    // binary flags. Split at `--` so they're inserted before it.
    eprintln!("=== Discovering test binaries ===");
    let cargo_args: Vec<_> = args.iter().take_while(|a| *a != "--").collect();
    let output = Command::new("cargo")
        .arg("test")
        .args(&cargo_args)
        .arg("--no-run")
        .arg("--message-format=json")
        .env("RUSTFLAGS", rustflags)
        .env("LLVM_PROFILE_FILE", profraw_pattern)
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .expect("failed to run cargo test --no-run");
    if !output.status.success() {
        eprintln!("cargo test --no-run failed with {}", output.status);
        std::process::exit(output.status.code().unwrap_or(1));
    }

    let jq_output = Command::new("jq")
        .arg("-r")
        .arg(r#"select(.profile.test == true) | .filenames[]"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(&output.stdout)?;
            child.wait_with_output()
        })
        .expect("failed to run jq — is it installed?");
    if !jq_output.status.success() {
        eprintln!("jq failed with {}", jq_output.status);
        std::process::exit(1);
    }

    let binaries: Vec<_> = String::from_utf8_lossy(&jq_output.stdout)
        .lines()
        .filter(|f| !f.contains("dSYM"))
        .map(|s| s.to_string())
        .collect();

    if binaries.is_empty() {
        eprintln!("error: no test binaries found");
        std::process::exit(1);
    }
    for b in &binaries {
        eprintln!("  found binary: {b}");
    }
    binaries
}

fn merge_profraw(llvm_profdata: &Path, profraw_dir: &Path, profdata_file: &Path) {
    eprintln!("=== Merging profraw files ===");
    let profraw_files: Vec<_> = fs::read_dir(profraw_dir)
        .expect("failed to read profraw dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension().is_some_and(|ext| ext == "profraw") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if profraw_files.is_empty() {
        eprintln!(
            "error: no .profraw files found in {}",
            profraw_dir.display()
        );
        std::process::exit(1);
    }
    eprintln!("  merging {} profraw files", profraw_files.len());

    let mut cmd = Command::new(llvm_profdata);
    cmd.arg("merge").arg("-sparse");
    for f in &profraw_files {
        cmd.arg(f);
    }
    cmd.arg("-o").arg(profdata_file);
    let status = cmd.status().expect("failed to run llvm-profdata");
    if !status.success() {
        eprintln!("llvm-profdata merge failed with {status}");
        std::process::exit(1);
    }
}

fn generate_report(llvm_cov: &Path, binaries: &[String], profdata_file: &Path, report_dir: &Path) {
    eprintln!("=== Generating HTML coverage report ===");
    if report_dir.exists() {
        fs::remove_dir_all(report_dir).expect("failed to clean report dir");
    }

    let mut cmd = Command::new(llvm_cov);
    cmd.arg("show")
        .arg("--format=html")
        .arg(format!("--output-dir={}", report_dir.display()))
        .arg("--ignore-filename-regex=/.cargo/registry")
        .arg("--ignore-filename-regex=/rustc/")
        .arg("--ignore-filename-regex=/.rustup/")
        .arg(format!("--instr-profile={}", profdata_file.display()))
        .arg("--show-line-counts-or-regions")
        .arg("--show-instantiations")
        .arg("--show-region-summary")
        .arg("--show-branch-summary");

    cmd.arg(&binaries[0]);
    for b in &binaries[1..] {
        cmd.arg("--object").arg(b);
    }

    if has_command("rustfilt") {
        cmd.arg("-Xdemangler=rustfilt");
    }

    let status = cmd.status().expect("failed to run llvm-cov");
    if !status.success() {
        eprintln!("llvm-cov show failed with {status}");
        std::process::exit(1);
    }

    eprintln!(
        "=== Coverage report written to {}/index.html ===",
        report_dir.display()
    );
}

fn find_repo_root() -> PathBuf {
    let mut dir = env::current_dir().expect("failed to get cwd");
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
            return dir;
        }
        if !dir.pop() {
            eprintln!("error: could not find wasmtime repo root");
            std::process::exit(1);
        }
    }
}

fn find_llvm_tool(name: &str) -> PathBuf {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .expect("failed to run rustc --print sysroot");
    let sysroot = String::from_utf8(output.stdout)
        .expect("non-utf8 sysroot")
        .trim()
        .to_string();

    let rustlib = Path::new(&sysroot).join("lib").join("rustlib");
    if let Ok(entries) = fs::read_dir(&rustlib) {
        for entry in entries.flatten() {
            let candidate = entry.path().join("bin").join(name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    eprintln!("warning: {name} not found in rustup sysroot, trying PATH");
    PathBuf::from(name)
}

fn has_command(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
