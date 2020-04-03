// Tell any dependencies, if necessary, where our WASI submodule is so they can
// use the same witx files if they want.
fn main() {
    let cwd = std::env::current_dir().unwrap();
    let wasi = cwd.join("WASI");
    println!("cargo:wasi={}", wasi.display());
    println!("cargo:rustc-env=WASI_ROOT={}", wasi.display());
}
