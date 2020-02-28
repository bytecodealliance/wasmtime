use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        pub(crate) use self::unix::*;
    } else if #[cfg(windows)] {
        mod windows;
        pub(crate) use self::windows::*;
    } else {
        compile_error!("wasi-common doesn't compile for this platform yet");
    }
}
