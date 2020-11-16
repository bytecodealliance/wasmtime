//! This build script:
//!  - has the configuration necessary for the wiggle and witx macros.

use std::path::PathBuf;

fn main() {
    // This is necessary for Wiggle/Witx macros.
    let wasi_root = PathBuf::from("./spec").canonicalize().unwrap();
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root.display());
}
