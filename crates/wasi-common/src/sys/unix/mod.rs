pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

mod dir;

#[cfg(any(
    target_os = "macos",
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "ios",
    target_os = "dragonfly"
))]
mod bsd;
#[cfg(target_os = "linux")]
mod linux;

use crate::{Error, Result};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::path::Path;

pub(crate) fn dev_null() -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .map_err(Into::into)
}

pub(crate) fn str_to_cstring(s: &str) -> Result<CString> {
    CString::new(s.as_bytes()).map_err(|_| Error::EILSEQ)
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> Result<File> {
    File::open(path).map_err(Into::into)
}
