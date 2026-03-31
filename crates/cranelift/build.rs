fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // When enabled, `cfg(gc_zeal)` activates aggressive GC debugging
    // assertions.
    println!("cargo:rustc-check-cfg=cfg(gc_zeal)");
}
