pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

use crate::host;
use crate::sys::errno_from_host;
use std::fs::File;
use std::path::Path;

pub(crate) fn dev_null() -> Result<File, host::__wasi_errno_t> {
    File::open("/dev/null")
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> Result<File, host::__wasi_errno_t> {
    File::open(path).map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}
