cfg_if::cfg_if! {
    if #[cfg(all(windows, any(target_arch = "x86_64", target_arch = "aarch64")))] {
        mod winx64;
        pub use self::winx64::*;
    } else if #[cfg(unix)] {
        mod systemv;
        pub use self::systemv::*;
    } else {
        compile_error!("unsupported target platform for unwind");
    }
}
