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

    // Check to see if we are on Unix and the `memory-init-cow` feature is
    // active. If so, enable the `memory_init_cow` rustc cfg so
    // `#[cfg(memory_init_cow)]` will work.
    let family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    let memory_init_cow = env::var("CARGO_FEATURE_MEMORY_INIT_COW").is_ok();
    if &family == "unix" && memory_init_cow {
        println!("cargo:rustc-cfg=memory_init_cow");
    }
}
