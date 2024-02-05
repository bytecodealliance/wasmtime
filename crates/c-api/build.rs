fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:include={dir}/include");
    println!("cargo:wasm_include={dir}/wasm-c-api/include");
}
