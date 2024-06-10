use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    set_commit_info_for_rustc();
}

fn set_commit_info_for_rustc() {
    if !Path::new(".git").exists() {
        return;
    }
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=9")
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return,
    };
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut parts = stdout.split_whitespace();
    let mut next = || parts.next().unwrap();
    println!("cargo:rustc-env=WASMTIME_GIT_HASH={}", next());
    println!(
        "cargo:rustc-env=WASMTIME_VERSION_INFO={} ({} {})",
        env!("CARGO_PKG_VERSION"),
        next(),
        next()
    );
}
