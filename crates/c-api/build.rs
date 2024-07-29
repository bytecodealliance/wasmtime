use cmake;
use std::{env, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let dst = cmake::Config::new(&dir).build_target("conf_h").build();

    println!("cargo:conf-include={}/build/include", dst.display());
    println!("cargo:include={}/include", dir.display());
}
