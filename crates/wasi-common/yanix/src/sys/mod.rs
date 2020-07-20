#[cfg(not(target_os = "wasi"))] // TODO: port this to WASI
use crate::dir::SeekLoc;
use cfg_if::cfg_if;
use std::io::Result;

cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android"))] {
        mod linux;
        pub(crate) use linux::*;
    } else if #[cfg(target_os = "emscripten")] {
        mod emscripten;
        pub(crate) use emscripten::*;
    } else if #[cfg(any(target_os = "macos",
                        target_os = "ios",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd",
                        target_os = "dragonfly"))] {
        mod bsd;
        pub(crate) use bsd::*;
    } else if #[cfg(target_os = "wasi")] {
        mod wasi;
        pub(crate) use wasi::*;
    } else if #[cfg(target_os = "fuchsia")] {
        mod fuchsia;
        pub(crate) use fuchsia::*;
    } else {
        compile_error!("yanix doesn't compile for this platform yet");
    }
}

#[cfg(not(target_os = "wasi"))] // TODO: port this to WASI
pub trait EntryExt {
    fn ino(&self) -> u64;
    fn seek_loc(&self) -> Result<SeekLoc>;
}
