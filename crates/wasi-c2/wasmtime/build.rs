fn main() {
    // wasi-c2's links & build.rs ensure this variable points to the wasi root:
    let wasi_root = std::env::var("DEP_WASI_C2_19_WASI").unwrap();
    // Make it available as WASI_ROOT:
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root);
}
