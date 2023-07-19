use std::env;
use wasmtime_versioned_export_macros::versioned_suffix;

fn main() {
    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{}", os), None);
    build.define(&format!("CFG_TARGET_ARCH_{}", arch), None);
    build.define("VERSIONED_SUFFIX", Some(versioned_suffix!()));
    if arch == "s390x" {
        println!("cargo:rerun-if-changed=src/trampolines/s390x.S");
        build.file("src/trampolines/s390x.S");
    }
    println!("cargo:rerun-if-changed=src/helpers.c");
    build.file("src/helpers.c");
    build.compile("wasmtime-helpers");
}
