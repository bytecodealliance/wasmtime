//! This module consists of helper types and functions for dealing
//! with setting the file times.

use crate::filesystem::primitives::{OpenOptions, open};
use rustix::io::Errno;
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
        Ok(file) => return fs::File::from(file).set_times(times),
        Err(err) => match Errno::from_io_error(&err) {
            Some(Errno::ACCESS) | Some(Errno::ISDIR) => (),
            _ => return Err(err),
        },
    }

    // Next try read.
    match open(start, path, OpenOptions::new().read(true)) {
        Ok(file) => return fs::File::from(file).set_times(times),
        Err(err) => match Errno::from_io_error(&err) {
            Some(Errno::ACCESS) => (),
            _ => return Err(err),
        },
    }

    // It's not possible to do anything else with generic POSIX. Plain
    // `utimensat` has two options:
    //  - Follow symlinks, which would open up a race in which a concurrent
    //    modification of the symlink could point outside the sandbox and we
    //    wouldn't be able to detect it, or
    //  - Don't follow symlinks, which would modify the timestamp of the symlink
    //    instead of the file we're trying to get to.
    //
    // So neither does what we need.
    Err(Errno::NOTSUP.into())
}
