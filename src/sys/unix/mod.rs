pub mod fdentry;
mod host_impl;
pub mod hostcalls;

use std::fs::File;
use std::io;
use std::path::Path;

pub fn dev_null() -> File {
    File::open("/dev/null").expect("failed to open /dev/null")
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(path)
}
