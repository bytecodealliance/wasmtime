fn main() {
    // Code in the `wasmtime` crate will use #[cfg(compiler)] conditional
    // compilation when runtime compilation is supported or not. This #[cfg] is
    // defined by this build script here, and is guarded with a conditional.
    // Currently this conditional is #[cfg(feature = "cranelift")] since that's
    // the only supported compiler.
    //
    // Note that #[doc(cfg)] throughout the `wasmtime` crate points here. We
    // want the rustdoc documentation to accurately reflect the requirements for
    // APIs, so the condition here is duplicated into all #[doc(cfg)]
    // attributes. If this condition is updated to emit #[cfg(compiler)] more
    // frequently then all rustdoc attributes also need to be updated with the
    // new condition to ensure the documentation accurately reflects when an API
    // is available.
    if cfg!(feature = "cranelift") || cfg!(feature = "winch") {
        println!("cargo:rustc-cfg=compiler");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
