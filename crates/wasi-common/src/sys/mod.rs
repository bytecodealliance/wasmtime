use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        pub(crate) use unix::*;
        pub use unix::preopen_dir;
    } else if #[cfg(windows)] {
        mod windows;
        pub(crate) use windows::*;
        pub use windows::preopen_dir;
    } else {
        compile_error!("wasi-common doesn't compile for this platform yet");
    }
}
