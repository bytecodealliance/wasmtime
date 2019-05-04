use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn wasi_sdk() -> PathBuf {
    Path::new(&env::var("WASI_SDK").unwrap_or("/opt/wasi-sdk".to_owned())).to_path_buf()
}

fn wasi_sysroot() -> PathBuf {
    match env::var("WASI_SYSROOT") {
        Ok(wasi_sysroot) => Path::new(&wasi_sysroot).to_path_buf(),
        Err(_) => {
            let mut path = wasi_sdk();
            path.push("share");
            path.push("sysroot");
            path
        }
    }
}

fn wasm_clang_root() -> PathBuf {
    match env::var("CLANG_ROOT") {
        Ok(clang) => Path::new(&clang).to_path_buf(),
        Err(_) => {
            let mut path = wasi_sdk();
            path.push("lib");
            path.push("clang");
            path.push("8.0.0");
            path
        }
    }
}

fn main() {
    let wasi_sysroot = wasi_sysroot();
    let wasm_clang_root = wasm_clang_root();
    assert!(
        wasi_sysroot.exists(),
        "wasi-sysroot not present at {:?}",
        wasi_sysroot
    );
    assert!(
        wasm_clang_root.exists(),
        "clang-root not present at {:?}",
        wasm_clang_root
    );

    let wasi_sysroot_core_h = wasi_sysroot.join("include/wasi/core.h");

    assert!(
        wasi_sysroot_core_h.exists(),
        "wasi-sysroot core.h not present at {:?}",
        wasi_sysroot_core_h
    );

    println!("cargo:rerun-if-changed={}", wasi_sysroot_core_h.display());

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let core_h_path = out_path.join("core.h");
    let core_h = File::create(&core_h_path).unwrap();

    // `bindgen` doesn't understand typed constant macros like `UINT8_C(123)`, so this fun regex
    // strips them off to yield a copy of `wasi/core.h` with bare constants.
    let sed_result = Command::new("sed")
        .arg("-E")
        .arg(r#"s/U?INT[0-9]+_C\(((0x)?[0-9]+)\)/\1/g"#)
        .arg(wasi_sysroot_core_h)
        .stdout(Stdio::from(core_h))
        .status()
        .expect("can execute sed");

    if !sed_result.success() {
        // something failed, but how?
        match sed_result.code() {
            Some(code) => panic!("sed failed with code {}", code),
            None => panic!("sed exited abnormally"),
        }
    }

    let host_builder = bindgen::Builder::default()
        .clang_arg("-nostdinc")
        .clang_arg("-D__wasi__")
        .clang_arg(format!("-isystem={}/include/", wasi_sysroot.display()))
        .clang_arg(format!("-I{}/include/", wasm_clang_root.display()))
        .header(core_h_path.to_str().unwrap())
        .whitelist_type("__wasi_.*")
        .whitelist_var("__WASI_.*");

    host_builder
        .generate()
        .expect("can generate host bindings")
        .write_to_file(out_path.join("wasi_host.rs"))
        .expect("can write host bindings");
}
