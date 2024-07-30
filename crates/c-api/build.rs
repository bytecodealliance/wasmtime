use cmake;
use std::{env, path::PathBuf};

// WASMTIME_FEATURE_LIST
const FEATURES: &[&str] = &[
    "ASYNC",
    "PROFILING",
    "CACHE",
    "PARALLEL_COMPILATION",
    "WASI",
    "LOGGING",
    "DISABLE_LOGGING",
    "COREDUMP",
    "ADDR2LINE",
    "DEMANGLE",
    "THREADS",
    "GC",
    "CRANELIFT",
    "WINCH",
];
// ... if you add a line above this be sure to change the other locations
// marked WASMTIME_FEATURE_LIST

fn main() {
    println!("cargo:rerun-if-changed=CMakesLists.txt");
    println!("cargo:rerun-if-changed=include/wasmtime/conf.h.in");
    let dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut config = cmake::Config::new(&dir);
    config
        .define("WASMTIME_DISABLE_ALL_FEATURES", "ON")
        .always_configure(true)
        .build_target("headers");
    for f in FEATURES {
        if env::var_os(format!("CARGO_FEATURE_{}", f)).is_some() {
            config.define(format!("WASMTIME_FEATURE_{}", f), "ON");
        }
    }
    let dst = config.build();

    println!("cargo:include={}/include", dst.display());
}
