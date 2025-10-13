use std::str;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // NB: duplicating a workaround in the wasmtime-fiber build script.
    custom_cfg("asan", cfg_is("sanitize", "address"));

    let unix = cfg("unix");
    let windows = cfg("windows");
    let miri = cfg("miri");

    // A boolean indicating whether there's a `sys` module for this platform.
    // This is true for `unix` or `windows`, but both of those require the `std`
    // feature to also be active so check that too.
    let supported_os = (unix || windows) && cfg!(feature = "std");

    // Determine if the current host architecture is supported by Cranelift
    // meaning that we might be executing native code.
    let has_host_compiler_backend = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" | "riscv64" | "s390x" | "aarch64" => true,
        _ => false,
    };

    let has_native_signals = !miri
        && (supported_os || cfg!(feature = "custom-native-signals"))
        && has_host_compiler_backend;
    let has_virtual_memory = supported_os || cfg!(feature = "custom-virtual-memory");
    let has_custom_sync = !cfg!(feature = "std") && cfg!(feature = "custom-sync-primitives");

    custom_cfg("has_native_signals", has_native_signals);
    custom_cfg("has_virtual_memory", has_virtual_memory);
    custom_cfg("has_custom_sync", has_custom_sync);
    custom_cfg("has_host_compiler_backend", has_host_compiler_backend);

    // If this OS isn't supported and no debug-builtins or if Cranelift doesn't support
    // the host or there's no need to build these helpers.
    #[cfg(feature = "runtime")]
    if has_host_compiler_backend && (supported_os || cfg!(feature = "debug-builtins")) {
        build_c_helpers();
    }

    // Figure out what to do about Pulley.
    //
    // If the target platform does not have any Cranelift support then Pulley
    // will be used by default. That means that the pulley feature is "enabled"
    // here and the default target is pulley. Note that by enabling the feature
    // here it doesn't actually enable the Cargo feature, it just passes a cfg
    // to rustc. That means that conditional dependencies enabled in
    // `Cargo.toml` (or other features) by `pulley` aren't activated, which is
    // why the `pulley` feature of this crate depends on nothing else.
    let default_target_pulley = !has_host_compiler_backend || miri;
    custom_cfg("default_target_pulley", default_target_pulley);
    if default_target_pulley {
        println!("cargo:rustc-cfg=feature=\"pulley\"");
    }
}

fn cfg(key: &str) -> bool {
    std::env::var(&format!("CARGO_CFG_{}", key.to_uppercase())).is_ok()
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

#[cfg(feature = "runtime")]
fn build_c_helpers() {
    use wasmtime_versioned_export_macros::versioned_suffix;

    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{os}"), None);
    build.define(&format!("CFG_TARGET_ARCH_{arch}"), None);
    build.define("VERSIONED_SUFFIX", Some(versioned_suffix!()));
    if std::env::var("CARGO_FEATURE_DEBUG_BUILTINS").is_ok() {
        build.define("FEATURE_DEBUG_BUILTINS", None);
    } else if cfg("windows") {
        // If debug builtins are disabled and this target is for Windows then
        // there's no need to build the C helpers file.
        //
        // TODO: should skip this on Unix targets as well but needs a solution
        // for `wasmtime_using_libunwind`.
        return;
    }

    println!("cargo:rerun-if-changed=src/runtime/vm/helpers.c");
    build.file("src/runtime/vm/helpers.c");
    build.compile("wasmtime-helpers");
}
