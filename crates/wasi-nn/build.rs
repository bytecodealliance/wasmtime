//! This build script:
//!  - has the configuration necessary for the wiggle and witx macros.

use std::path::PathBuf;

fn main() {
    // This is necessary for Wiggle/Witx macros.
    let wasi_root = PathBuf::from("./spec").canonicalize().unwrap();
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root.display());

    // Also automatically rebuild if the Witx files change
    for entry in walkdir::WalkDir::new(wasi_root) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
