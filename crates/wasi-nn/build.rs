//! This build script:
//!  - has the configuration necessary for the Wiggle and WITX macros.
fn main() {
    // This is necessary for Wiggle/WITX macros.
    let cwd = std::env::current_dir().unwrap();
    let wasi_root = cwd.join("witx");
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root.display());

    // Also automatically rebuild if the WITX files change
    for entry in walkdir::WalkDir::new(wasi_root) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
