use crate::filesystem::primitives::{OpenOptions, OpenOptionsExt, open};
use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
};

#[inline]
pub(crate) fn set_times_impl(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    set_times_inner(start, path, atime, mtime, 0)
}

#[inline]
pub(crate) fn set_times_nofollow_impl(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    set_times_inner(start, path, atime, mtime, FILE_FLAG_OPEN_REPARSE_POINT)
}

fn set_times_inner(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
    custom_flags: u32,
) -> io::Result<()> {
    let custom_flags = custom_flags | FILE_FLAG_BACKUP_SEMANTICS;

    // On Windows, `set_times` requires write permissions.
    let file = open(
        start,
        path,
        OpenOptions::new().write(true).custom_flags(custom_flags),
    )?;
    let mut times = fs::FileTimes::new();
    if let Some(atime) = atime {
        times = times.set_accessed(atime);
    }
    if let Some(mtime) = mtime {
        times = times.set_modified(mtime);
    }
    file.set_times(times)
}
