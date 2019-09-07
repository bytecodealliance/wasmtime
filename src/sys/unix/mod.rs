pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

use crate::Result;
use std::fs::File;
use std::path::Path;

pub(crate) fn dev_null() -> Result<File> {
    File::open("/dev/null").map_err(Into::into)
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> Result<File> {
    File::open(path).map_err(Into::into)
}
