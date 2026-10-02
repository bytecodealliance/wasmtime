use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    set_commit_info_for_rustc();

    println!("cargo:rustc-check-cfg=cfg(asan)");
    match env::var("CARGO_CFG_SANITIZE") {
        Ok(s) if s == "address" => {
            println!("cargo:rustc-cfg=asan");
        }
        _ => {}
    }

    // Mirror the `has_mmu_interruption` cfg in `crates/wasmtime/build.rs`.
    //
    // We can omit the `std` check here, because `wasmtime` is always built with
    // `std` from this crate.
    println!("cargo:rustc-check-cfg=cfg(has_mmu_interruption)");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    if target_os == "linux"
        && (target_arch == "x86_64" || target_arch == "aarch64")
        && cfg!(feature = "cranelift")
        && cfg!(feature = "async")
    {
        println!("cargo:rustc-cfg=has_mmu_interruption");
    }
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
