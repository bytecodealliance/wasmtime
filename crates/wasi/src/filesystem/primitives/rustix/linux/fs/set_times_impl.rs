//! This module consists of helper types and functions for dealing
//! with setting the file times specific to Linux.

use super::procfs::set_times_through_proc_self_fd;
use crate::filesystem::primitives::{OpenOptions, open};
use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};

pub(crate) fn set_times_impl(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    let mut times = fs::FileTimes::new();
    if let Some(atime) = atime {
        times = times.set_accessed(atime);
    }
    if let Some(mtime) = mtime {
        times = times.set_modified(mtime);
    }
    // Try `futimens` with a normal handle. Normal handles need some kind of
    // access, so first try write.
    match open(start, path, OpenOptions::new().write(true)) {
        Ok(file) => return file.set_times(times),
        Err(err) => match rustix::io::Errno::from_io_error(&err) {
            Some(rustix::io::Errno::ACCESS) | Some(rustix::io::Errno::ISDIR) => (),
            _ => return Err(err),
        },
    }

    // Next try read.
    match open(start, path, OpenOptions::new().read(true)) {
        Ok(file) => return file.set_times(times),
        Err(err) => match rustix::io::Errno::from_io_error(&err) {
            Some(rustix::io::Errno::ACCESS) => (),
            _ => return Err(err),
        },
    }

    // If neither of those worked, turn to `/proc`.
    set_times_through_proc_self_fd(start, path, atime, mtime)
}
