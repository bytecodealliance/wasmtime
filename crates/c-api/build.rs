fn main() {
    // Set SONAME in shared library on Linux
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libwasmtime.so");
    }
}
