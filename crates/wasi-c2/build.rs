// Tell any dependencies, if necessary, where our WASI submodule is so they can
// use the same witx files if they want.
fn main() {
    let cwd = std::env::current_dir().unwrap();
    let wasi = cwd.join("..").join("wasi-common").join("WASI");
    println!("cargo:wasi={}", wasi.display());
    println!("cargo:rustc-env=WASI_ROOT={}", wasi.display());

    match rustc_version::version_meta()
        .expect("query rustc release channel")
        .channel
    {
        rustc_version::Channel::Nightly => {
            println!("cargo:rustc-cfg=nightly");
        }
        _ => {}
    }
}
