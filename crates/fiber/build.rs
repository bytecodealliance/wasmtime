use std::env;
use std::fs;

fn main() {
    let mut build = cc::Build::new();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let family_file = format!("src/arch/{}.c", family);
    let arch_file = format!("src/arch/{}.S", arch);
    if fs::metadata(&family_file).is_ok() {
        build.file(&family_file);
    } else if fs::metadata(&arch_file).is_ok() {
        build.file(&arch_file);
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
