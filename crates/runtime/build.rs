use std::{env, fs, path::PathBuf};
use wasmtime_versioned_export_macros::versioned_suffix;

fn main() {
    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{}", os), None);
    build.define(&format!("CFG_TARGET_ARCH_{}", arch), None);
    if arch == "s390x" {
        println!("cargo:rerun-if-changed=src/trampolines/s390x.S");

        // cc does not preprocess macros passed with -D for `.s` files so need to do
        // it manually
        let asm = fs::read_to_string("src/trampolines/s390x.S").unwrap();
        let asm = asm.replace("VERSIONED_SUFFIX", versioned_suffix!());
        let out_dir = env::var("OUT_DIR").unwrap();
        let file_path = PathBuf::from(out_dir).join("s390x_preprocessed.S");
        fs::write(&file_path, asm).unwrap();
        build.file(file_path);
    }
    println!("cargo:rerun-if-changed=src/helpers.c");
    build.file("src/helpers.c");
    build.compile("wasmtime-helpers");
}
