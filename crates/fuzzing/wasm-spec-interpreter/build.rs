/// Build the OCaml code and statically link it into the Rust library; see the
/// [ocaml-interop
/// example](https://github.com/tezedge/ocaml-interop/blob/master/testing/rust-caller/build.rs)
/// for more details. After playing with this a bit, I discovered that the best
/// approach to avoid missing symbols was to imitate `dune`: I observed `rm -rf
/// _build && dune build ./ocaml/interpret.exe.o --display=verbose` and used
/// that as a pattern, now encoded in `ocaml/Makefile` for easier debugging.
use std::{env, path::PathBuf, process::Command};

const LIB_NAME: &'static str = "interpret";
const OCAML_DIR: &'static str = "ocaml";
const SPEC_DIR: &'static str = "ocaml/spec";
const SPEC_REPOSITORY: &'static str = "https://github.com/conrad-watt/spec";
const SPEC_REPOSITORY_BRANCH: &'static str = "wasmtime_fuzzing";
const SPEC_REPOSITORY_REV: &'static str = "c6bab4461e10229e557aae2e1027cadfce0161ce";

fn main() {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"has-libinterpret\"))");
    println!("cargo:rustc-check-cfg=cfg(fuzzing)");
    if cfg!(feature = "build-libinterpret") {
        build();
    }
}

fn build() {
    let out_dir = &env::var("OUT_DIR").unwrap();

    // Re-run if changed.
    println!("cargo:rerun-if-changed={OCAML_DIR}/{LIB_NAME}.ml");
    println!("cargo:rerun-if-changed={OCAML_DIR}/Makefile");

    if let Some(other_dir) = env::var_os("FFI_LIB_DIR") {
        // Link with a library provided in the `FFI_LIB_DIR`.
        println!("cargo:rustc-link-search={}", other_dir.to_str().unwrap());
        println!("cargo:rustc-link-lib=static={LIB_NAME}");
    } else {
        // Ensure the spec repository is present.
        if is_spec_repository_empty(SPEC_DIR) {
            retrieve_spec_repository(SPEC_DIR)
        }

        // Build the library to link to.
        build_lib(out_dir, OCAML_DIR);
        println!("cargo:rustc-link-search={out_dir}");
        println!("cargo:rustc-link-lib=static={LIB_NAME}");
    }

    // Enabling this feature alerts the compiler to use the `with_library`
    // module.
    println!("cargo:rustc-cfg=feature=\"has-libinterpret\"");
}

// Build the OCaml library into Cargo's `out` directory.
fn build_lib(out_dir: &str, ocaml_dir: &str) {
    let status = Command::new("make")
        .arg(format!("BUILD_DIR={out_dir}"))
        .current_dir(ocaml_dir)
        .status()
        .expect("Failed to execute 'make' command to build OCaml library");

    assert!(
        status.success(),
        "Failed to build the OCaml library using 'make'."
    )
}

// Check if the spec repository directory contains any files.
fn is_spec_repository_empty(destination: &str) -> bool {
    PathBuf::from(destination)
        .read_dir()
        .map(|mut i| i.next().is_none())
        .unwrap_or(true)
}

// Clone the spec repository into `destination`. This exists due to the large
// size of the dependencies (e.g. KaTeX) that are pulled if this were cloned
// recursively as a submodule.
fn retrieve_spec_repository(destination: &str) {
    let status = Command::new("git")
        .arg("clone")
        .arg(SPEC_REPOSITORY)
        .arg("-b")
        .arg(SPEC_REPOSITORY_BRANCH)
        .arg(destination)
        .status()
        .expect("Failed to execute 'git' command to clone spec repository.");
    assert!(status.success(), "Failed to retrieve the spec repository.");

    let status = Command::new("git")
        .arg("reset")
        .arg("--hard")
        .arg(SPEC_REPOSITORY_REV)
        .current_dir(destination)
        .status()
        .expect("Failed to execute 'git' command to clone spec repository.");
    assert!(status.success(), "Failed to reset to revision.");
}
