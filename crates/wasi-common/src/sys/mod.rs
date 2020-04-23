pub(crate) mod clock;
pub(crate) mod fd;
pub(crate) mod oshandle;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        use unix as sys_impl;
        pub use unix::preopen_dir;
    } else if #[cfg(windows)] {
        mod windows;
        use windows as sys_impl;
        pub use windows::preopen_dir;
    } else {
        compile_error!("wasi-common doesn't compile for this platform yet");
    }
}

pub(crate) use sys_impl::path;
pub(crate) use sys_impl::poll;
