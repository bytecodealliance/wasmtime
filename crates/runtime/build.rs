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
}
