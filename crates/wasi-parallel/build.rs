//! This build script:
//!  - has the configuration necessary for the wiggle and witx macros
//!  - generates Wasm from the files in `tests/rust` to `tests/wasm`

#[cfg(feature = "build-tests")]
use std::{
    fs::DirBuilder,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    // This is necessary for wiggle/witx macros.
    let cwd = std::env::current_dir().unwrap();
    let wasi_root = cwd.join("spec");
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root.display());

    // Also automatically rebuild if the WITX files change.
    for entry in walkdir::WalkDir::new(wasi_root) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }

    // If requested, rebuild the test files.
    #[cfg(feature = "build-tests")]
    {
        build_wasm("tests");
        build_wasm("benches");
    }
}

#[cfg(feature = "build-tests")]
fn build_wasm<P: AsRef<Path>>(root: P) {
    let root_dir = Path::new(root.as_ref().as_os_str());
    let wasm_dir = root_dir.join("wasm");

    DirBuilder::new().recursive(true).create(&wasm_dir).unwrap();

    // Automatically rebuild any Rust tests.
    if root_dir.join("rust").exists() {
        for entry in walkdir::WalkDir::new(root_dir.join("rust")) {
            let entry = entry.unwrap();
            println!("cargo:rerun-if-changed={}", entry.path().display());
            if entry.path().is_file() && entry.file_name() != "wasi_parallel.rs" {
                compile_rust(entry.path(), &wasm_dir)
            }
        }
    }
}

/// Use rustc to compile a Rust file to a Wasm file that uses the wasi-parallel
/// API.
#[cfg(feature = "build-tests")]
fn compile_rust<P1: AsRef<Path>, P2: AsRef<Path>>(source_file: P1, destination_dir: P2) {
    let stem = source_file.as_ref().file_stem().unwrap();
    let mut destination_file: PathBuf = [destination_dir.as_ref().as_os_str(), stem]
        .iter()
        .collect();
    destination_file.set_extension("wasm");

    let mut command = Command::new("rustc");
    command
        .arg("--target")
        .arg("wasm32-wasi")
        .arg(source_file.as_ref().to_str().unwrap())
        .arg("-o")
        .arg(destination_file.to_str().unwrap());

    let status = command
        .status()
        .expect("Failed to execute 'rustc' command to generate Wasm file.");

    assert!(
        status.success(),
        "Failed to compile test program: {:?}",
        command
    )
}
