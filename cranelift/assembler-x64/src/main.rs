//! Print the path to the generated code.
#![cfg_attr(feature = "core", no_std)]

#[cfg(not(feature = "core"))]
fn main() {
    let paths: Vec<std::path::PathBuf> = include!(concat!(env!("OUT_DIR"), "/generated-files.rs"));
    for path in paths {
        println!("{}", path.display());
    }
}

#[cfg(feature = "core")]
fn main() {}

#[cfg(feature = "core")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}