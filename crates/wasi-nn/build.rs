//! This build script:
//!  - has the configuration necessary for the wiggle and witx macros.
fn main() {
    // This is necessary for Wiggle/Witx macros.
    let cwd = std::env::current_dir().unwrap();
    let wasi_root = cwd.join("spec");
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root.display());

    // Also automatically rebuild if the Witx files change
    for entry in walkdir::WalkDir::new(wasi_root) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
