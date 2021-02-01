use std::env;

fn main() {
    let mut build = cc::Build::new();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if family == "windows" {
        build.file("src/arch/windows.c");
    } else if arch == "x86_64" {
        build.file("src/arch/x86_64.S");
    } else if arch == "x86" {
        build.file("src/arch/x86.S");
    } else if arch == "aarch64" {
        build.file("src/arch/aarch64.S");
    } else {
        panic!(
            "wasmtime doesn't support fibers on platform: {}",
            env::var("TARGET").unwrap()
        );
    }
    build.define(&format!("CFG_TARGET_OS_{}", os), None);
    build.define(&format!("CFG_TARGET_ARCH_{}", arch), None);
    build.compile("wasmtime-fiber");
}
