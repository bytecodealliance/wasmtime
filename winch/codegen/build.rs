fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    enable_default_arch();

    // Pretend that Winch has these features. These are injected via the
    // `wasmtime_environ::foreach_builtin_function!` macro so to ensure that we
    // don't get spurious warnings about unused cfg's these are printed out
    // here.
    faux_feature("stack-switching");
    faux_feature("gc");
    faux_feature("gc-copying");
    faux_feature("gc-null");
    faux_feature("gc-drc");
    faux_feature("threads");
    faux_feature("wmemcheck");
    faux_feature("component-model");
    faux_feature("incremental-cache");
}

fn faux_feature(feature: &str) {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"{feature}\"))");
    println!("cargo:rustc-cfg=feature=\"{feature}\"");
}

fn enable_default_arch() {
    if cfg!(feature = "x64") || cfg!(feature = "arm64") || cfg!(feature = "all-arch") {
        return;
    }

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if arch == "x86_64" {
        println!("cargo:rustc-cfg=feature=\"x64\"");
    } else if arch == "aarch64" {
        println!("cargo:rustc-cfg=feature=\"arm64\"");
    } else {
        println!("cargo:rustc-cfg=feature=\"{arch}\"");
    }
}
