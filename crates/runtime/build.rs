use std::env;

fn main() {
    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{}", os), None);
    build.define(&format!("CFG_TARGET_ARCH_{}", arch), None);
    if arch == "s390x" {
        println!("cargo:rerun-if-changed=src/trampolines/s390x.S");
        build.file("src/trampolines/s390x.S");
    }
    println!("cargo:rerun-if-changed=src/helpers.c");
    build.file("src/helpers.c");
    build.compile("wasmtime-helpers");

    // Check to see if we are on Unix and the `memory-init-cow` feature is
    // active. If so, enable the `memory_init_cow` rustc cfg so
    // `#[cfg(memory_init_cow)]` will work.
    let family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    let memory_init_cow = env::var("CARGO_FEATURE_MEMORY_INIT_COW").is_ok();
    if &family == "unix" && memory_init_cow {
        println!("cargo:rustc-cfg=memory_init_cow");
    }
}
