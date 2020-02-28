pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

use crate::old::snapshot_0::Result;
use std::fs::{File, OpenOptions};

pub(crate) fn dev_null() -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("NUL")
        .map_err(Into::into)
}
