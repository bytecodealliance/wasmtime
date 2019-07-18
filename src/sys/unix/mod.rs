pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

use crate::sys::errno_from_host;
use crate::{host, Result};
use std::fs::File;
use std::path::Path;

pub(crate) fn dev_null() -> Result<File> {
    File::open("/dev/null")
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> Result<File> {
    File::open(path).map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}
