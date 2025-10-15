use std::env;
use std::path::Path;
use std::process::Command;
use std::process::exit;

#[allow(dead_code)]
fn build_c_plugin() {
    // Run wit-bindgen
    let mut status = Command::new("wit-bindgen")
        .args(["c", "../wit/calculator.wit"])
        .current_dir("c-plugin")
        .status()
        .expect("wit-bindgen c ../wit/calculator.wit failed (cwd = c-plugin)");
    if !status.success() {
        println!(
            "wit-bindgen c ../wit/calculator.wit failed (cwd = c-plugin): status {}",
            status
        );
        exit(1);
    }

    // Check that wasi-sdk is installed
    let wasi_sdk_path = env::var("WASI_SDK_PATH");
    if wasi_sdk_path.is_err() {
        println!("You must set WASI_SDK_PATH to the location where you installed the WASI SDK");
        exit(1);
    }
    // Compile the plugin
    let clang_path = Path::new(&wasi_sdk_path.unwrap()).join("bin/wasm32-wasip2-clang");
    status = Command::new(&clang_path)
        .args([
            "-o",
            "add.wasm",
            "-mexec-model=reactor",
            "add.c",
            "plugin.c",
            "plugin_component_type.o",
        ])
        .current_dir("c-plugin")
        .status()
        .unwrap_or_else(|_| {
            panic!(
                "{} failed to compile the plugin (cwd = c-plugin)",
                clang_path.display()
            )
        });
    if !status.success() {
        println!(
            "{} failed to compile the plugin (cwd = c-plugin): status {}",
            clang_path.display(),
            status
        );
        exit(1);
    }
}

#[allow(dead_code)]
fn build_js_plugin() {
    Command::new("jco")
        .args([
            "componentize",
            "--wit",
            "../wit/calculator.wit",
            "--world-name",
            "plugin",
            "--out",
            "subtract.wasm",
            "--disable=all",
            "subtract.js",
        ])
        .current_dir("js-plugin")
        .status()
        .expect("jco componentize failed (is jco installed?) (cwd = js-plugin)");
}

fn main() {
    #[cfg(feature = "build-plugins")]
    build_c_plugin();
    #[cfg(feature = "build-plugins")]
    build_js_plugin();
}
