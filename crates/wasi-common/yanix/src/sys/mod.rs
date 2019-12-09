use crate::dir::SeekLoc;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android",
                 target_os = "emscripten"))] {
        mod linux;
        pub(crate) use self::linux::*;
    }
    else if #[cfg(any(target_os = "macos",
                      target_os = "ios",
                      target_os = "freebsd",
                      target_os = "netbsd",
                      target_os = "openbsd",
                      target_os = "dragonfly"))] {
        mod bsd;
        pub(crate) use self::bsd::*;
    } else {
        compile_error!("yanix doesn't compile for this platform yet");
    }
}

pub trait EntryExt {
    fn ino(&self) -> u64;
    fn seek_loc(&self) -> SeekLoc;
}
