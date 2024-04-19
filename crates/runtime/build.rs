use std::env;
use wasmtime_versioned_export_macros::versioned_suffix;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // NB: duplicating a workaround in the wasmtime-fiber build script.
    match env::var("CARGO_CFG_SANITIZE") {
        Ok(s) if s == "address" => {
            println!("cargo:rustc-cfg=asan");
        }
        _ => {}
    }

    // If this platform is neither unix nor windows then there's no default need
    // for a C helper library since `helpers.c` is tailored for just these
    // platforms currently.
    if env::var("CARGO_CFG_UNIX").is_err() && env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }

    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{}", os), None);
    build.define(&format!("CFG_TARGET_ARCH_{}", arch), None);
    build.define("VERSIONED_SUFFIX", Some(versioned_suffix!()));
    println!("cargo:rerun-if-changed=src/helpers.c");
    build.file("src/helpers.c");
    build.compile("wasmtime-helpers");
}
