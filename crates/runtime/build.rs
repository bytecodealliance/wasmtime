use std::env;

fn main() {
    println!("cargo:rerun-if-changed=src/helpers.c");
    cc::Build::new()
        .warnings(true)
        .define(
            &format!("CFG_TARGET_OS_{}", env::var("CARGO_CFG_TARGET_OS").unwrap()),
            None,
        )
        .file("src/helpers.c")
        .compile("wasmtime-helpers");

    // Check to see if we are on Linux and the `memfd` feature is
    // active. If so, enable the `memfd` rustc cfg so `#[cfg(memfd)]`
    // will work.
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let is_memfd = env::var("CARGO_FEATURE_MEMFD").is_ok();
    let is_uffd = env::var("CARGO_FEATURE_UFFD").is_ok();
    if &os == "linux" && is_memfd && !is_uffd {
        println!("cargo:rustc-cfg=memfd");
    }
}
