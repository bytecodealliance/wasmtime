pub(crate) mod entry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

use std::fs::{File, OpenOptions};
use std::io::Result;

pub(crate) fn dev_null() -> Result<File> {
    OpenOptions::new().read(true).write(true).open("NUL")
}
