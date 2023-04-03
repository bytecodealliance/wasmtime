fn main() {
    if cfg!(feature = "x64") || cfg!(feature = "arm64") || cfg!(feature = "all-arch") {
        return;
    }

    if cfg!(target_arch = "x86_64") {
        println!("cargo:rustc-cfg=feature=\"x64\"");
    } else if cfg!(target_arch = "aarch64") {
        println!("cargo:rustc-cfg=feature=\"arm64\"");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
