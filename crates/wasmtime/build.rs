fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // NB: duplicating a workaround in the wasmtime-fiber build script.
    println!("cargo:rustc-check-cfg=cfg(asan)");
    if cfg_is("sanitize", "address") {
        println!("cargo:rustc-cfg=asan");
    }

    let unix = cfg("unix");
    let windows = cfg("windows");
    let miri = cfg("miri");
    let supported_platform = unix || windows;

    let has_native_signals =
        !miri && (supported_platform || cfg!(feature = "custom-native-signals"));
    let has_virtual_memory = supported_platform || cfg!(feature = "custom-virtual-memory");

    println!("cargo:rustc-check-cfg=cfg(has_native_signals, has_virtual_memory)");
    if has_native_signals {
        println!("cargo:rustc-cfg=has_native_signals");
    }
    if has_virtual_memory {
        println!("cargo:rustc-cfg=has_virtual_memory");
    }

    #[cfg(feature = "runtime")]
    build_c_helpers();

    // Flag pulley as enabled unconditionally on 32-bit targets to ensure that
    // wasm is runnable by default like it is on other 64-bit native platforms.
    // Note that this doesn't actually enable the Cargo feature, it just changes
    // the cfg's passed to the crate, so for example care is still taken in
    // `Cargo.toml` to handle pulley-specific dependencies on 32-bit platforms.
    let target_pointer_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    if target_pointer_width == "32" {
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

#[cfg(feature = "runtime")]
fn build_c_helpers() {
    use wasmtime_versioned_export_macros::versioned_suffix;

    // If this platform is neither unix nor windows then there's no default need
    // for a C helper library since `helpers.c` is tailored for just these
    // platforms currently.
    if !cfg("unix") && !cfg("windows") {
        return;
    }

    let mut build = cc::Build::new();
    build.warnings(true);
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    build.define(&format!("CFG_TARGET_OS_{os}"), None);
    build.define(&format!("CFG_TARGET_ARCH_{arch}"), None);
    build.define("VERSIONED_SUFFIX", Some(versioned_suffix!()));
    if std::env::var("CARGO_FEATURE_DEBUG_BUILTINS").is_ok() {
        build.define("FEATURE_DEBUG_BUILTINS", None);
    }

    // On MinGW targets work around a bug in the MinGW compiler described at
    // https://github.com/bytecodealliance/wasmtime/pull/9688#issuecomment-2573367719
    if cfg("windows") && cfg_is("target_env", "gnu") {
        build.define("__USE_MINGW_SETJMP_NON_SEH", None);
    }

    println!("cargo:rerun-if-changed=src/runtime/vm/helpers.c");
    build.file("src/runtime/vm/helpers.c");
    build.compile("wasmtime-helpers");
}
