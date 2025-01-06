fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(feature = "runtime")]
    build_c_helpers();
}

#[cfg(feature = "runtime")]
fn build_c_helpers() {
    use wasmtime_versioned_export_macros::versioned_suffix;

    // NB: duplicating a workaround in the wasmtime-fiber build script.
    println!("cargo:rustc-check-cfg=cfg(asan)");
    match std::env::var("CARGO_CFG_SANITIZE") {
        Ok(s) if s == "address" => {
            println!("cargo:rustc-cfg=asan");
        }
        _ => {}
    }

    // If this platform is neither unix nor windows then there's no default need
    // for a C helper library since `helpers.c` is tailored for just these
    // platforms currently.
    if std::env::var("CARGO_CFG_UNIX").is_err() && std::env::var("CARGO_CFG_WINDOWS").is_err() {
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
    if std::env::var("CARGO_CFG_WINDOWS").is_ok()
        && std::env::var("CARGO_CFG_TARGET_ENV").ok().as_deref() == Some("gnu")
    {
        build.define("__USE_MINGW_SETJMP_NON_SEH", None);
    }

    println!("cargo:rerun-if-changed=src/runtime/vm/helpers.c");
    build.file("src/runtime/vm/helpers.c");
    build.compile("wasmtime-helpers");
}
