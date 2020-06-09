fn main() {
    let wasi_root = std::env::var("DEP_WASI_COMMON_18_WASI").unwrap();
    println!("cargo:rustc-env=WASI_ROOT={}", wasi_root);
}
