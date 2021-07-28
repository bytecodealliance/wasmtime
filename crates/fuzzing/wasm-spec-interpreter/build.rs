/// Build the OCaml code and statically link it into the Rust library; see the
/// [ocaml-interop
/// example](https://github.com/tezedge/ocaml-interop/blob/master/testing/rust-caller/build.rs)
/// for more details. After playing with this a bit, I discovered that the best
/// approach to avoid missing symbols was to imitate `dune`: I observed `rm -rf
/// _build && dune build ./ocaml/interpret.exe.o --display=verbose` and used
/// that as a pattern, now encoded in `ocaml/Makefile` for easier debugging.
use std::{env, process::Command};

const LIB_NAME: &'static str = "interpret";
const OCAML_DIR: &'static str = "ocaml";

fn main() {
    if cfg!(feature = "build-libinterpret") {
        build();
    }
}

fn build() {
    let out_dir = &env::var("OUT_DIR").unwrap();

    // Re-run if changed.
    println!("cargo:rerun-if-changed={}/{}.ml", OCAML_DIR, LIB_NAME);
    println!("cargo:rerun-if-changed={}/Makefile", OCAML_DIR);

    if let Some(other_dir) = env::var_os("FFI_LIB_DIR") {
        // Link with a library provided in the `FFI_LIB_DIR`.
        println!("cargo:rustc-link-search={}", other_dir.to_str().unwrap());
        println!("cargo:rustc-link-lib=static={}", LIB_NAME);
    } else {
        // Build the library to link to.
        build_lib(out_dir, OCAML_DIR);
        println!("cargo:rustc-link-search={}", out_dir);
        println!("cargo:rustc-link-lib=static={}", LIB_NAME);
    }

    // Enabling this feature alerts the compiler to use the `with_library`
    // module.
    println!("cargo:rustc-cfg=feature=\"has-libinterpret\"");
}

// Build the OCaml library into Cargo's `out` directory.
fn build_lib(out_dir: &str, ocaml_dir: &str) {
    let status = Command::new("make")
        .arg(format!("BUILD_DIR={}", out_dir))
        .current_dir(ocaml_dir)
        .status()
        .expect("Failed to execute 'make' command to build OCaml library");

    assert!(
        status.success(),
        "Failed to build the OCaml library using 'make'."
    )
}
