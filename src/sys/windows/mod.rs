pub mod fdentry;
mod host_impl;
pub mod hostcalls;

use std::fs::File;

pub fn dev_null() -> File {
    File::open("NUL").expect("failed to open NUL")
}
