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

    // Check to see if we are on Unix and the `memfd` feature is
    // active. If so, enable the `memfd` rustc cfg so `#[cfg(memfd)]`
    // will work.
    //
    // Note that while this is called memfd it only actually uses the `memfd`
    // crate on Linux and on other Unix platforms this tries to reuse mmap'd
    // `*.cwasm` files.
    let family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    let is_memfd = env::var("CARGO_FEATURE_MEMFD").is_ok();
    let is_uffd = env::var("CARGO_FEATURE_UFFD").is_ok();
    if &family == "unix" && is_memfd && !is_uffd {
        println!("cargo:rustc-cfg=memfd");
    }
}
