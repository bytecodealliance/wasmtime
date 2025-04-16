//! This build script:
//!  - has the configuration necessary for the Wiggle and WITX macros.
fn main() {
    let cwd = std::env::current_dir().unwrap();
    let wasi_root = cwd.join("witx");

    // Also automatically rebuild if the WITX files change
    for entry in walkdir::WalkDir::new(wasi_root) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
