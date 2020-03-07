pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
        use self::linux as sys_impl;
    } else if #[cfg(target_os = "emscripten")] {
        mod emscripten;
        use self::emscripten as sys_impl;
    } else if #[cfg(any(target_os = "macos",
                        target_os = "netbsd",
                        target_os = "freebsd",
                        target_os = "openbsd",
                        target_os = "ios",
                        target_os = "dragonfly"))] {
        mod bsd;
        use self::bsd as sys_impl;
    }
}

use crate::old::snapshot_0::Result;
use std::fs::{File, OpenOptions};

pub(crate) fn dev_null() -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .map_err(Into::into)
}
