// A small build script to include the contents of the wast test suite into the
// final fuzzing binary so the fuzzing binary can be run elsewhere and doesn't
// rely on the original source tree.

use std::env;
use std::path::PathBuf;
use wasmtime_test_util::wast::WastTest;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(arc_try_new)");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut root = env::current_dir().unwrap();
    root.pop(); // chop off 'fuzzing'
    root.pop(); // chop off 'crates'

    let mut tests = wasmtime_test_util::wast::find_tests(&root).unwrap();
    tests.sort_by_key(|test| test.path.clone());

    let mut code = format!("static FILES: &[fn() -> wasmtime_test_util::wast::WastTest] = &[\n");

    for test in tests {
        let WastTest {
            path,
            contents: _,
            config,
        } = test;
        println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        code.push_str(&format!(
            "|| {{
                wasmtime_test_util::wast::WastTest {{
                    path: {path:?}.into(),
                    contents: include_str!({path:?}).into(),
                    config: wasmtime_test_util::wast::{config:?},
                }}
            }},"
        ));
    }

    code.push_str("];\n");
    std::fs::write(out_dir.join("wasttests.rs"), code).unwrap();

    // NB: duplicating a workaround in the wasmtime-fiber build script.
    custom_cfg("asan", cfg_is("sanitize", "address"));

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // Keep this in sync with the platforms listed in `Cargo.toml` that the
    // `v8` dependency is enabled for.
    let v8_supported = match target_os.as_str() {
        "linux" => target_arch == "x86_64" || target_arch == "aarch64",
        "macos" => target_arch == "aarch64",
        _ => false,
    };
    let v8 = v8_supported && env::var_os("CARGO_FEATURE_V8").is_some();
    custom_cfg("v8", v8);
}

fn cfg_is(key: &str, val: &str) -> bool {
    std::env::var(&format!("CARGO_CFG_{}", key.to_uppercase()))
        .ok()
        .as_deref()
        == Some(val)
}

fn custom_cfg(key: &str, enabled: bool) {
    println!("cargo:rustc-check-cfg=cfg({key})");
    if enabled {
        println!("cargo:rustc-cfg={key}");
    }
}
