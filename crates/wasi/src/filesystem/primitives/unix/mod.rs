use crate::filesystem::primitives::{MaybeOwnedFile, OpenOptions, open, open_parent};
use rustix::fs::{AtFlags, utimensat};
use rustix::io::Errno;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod create_dir_unchecked;
mod dir_entry_inner;
mod dir_utils;
mod file_type_ext;
mod hard_link_unchecked;
mod is_same_file;
mod metadata_ext;
mod oflags;
mod open_options_ext;
mod open_unchecked;
mod read_dir_inner;
mod read_link_unchecked;
mod remove_dir_unchecked;
mod remove_file_unchecked;
mod rename_unchecked;
mod stat_unchecked;
mod symlink_unchecked;
mod times;

pub(crate) mod errors;

// On FreeBSD, use optimized implementations based on
// `O_RESOLVE_BENEATH`/`AT_RESOLVE_BENEATH` and `O_PATH` when available.
#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "freebsd")]
pub(crate) use self::freebsd::*;

// On Linux, use optimized implementations based on
// `openat2` and `O_PATH` when available.
#[cfg(any(target_os = "android", target_os = "linux"))]
mod linux;
#[cfg(any(target_os = "android", target_os = "linux"))]
pub(crate) use self::linux::*;

pub(crate) use create_dir_unchecked::create_dir_unchecked;
pub(crate) use dir_entry_inner::DirEntryInner;
pub(crate) use dir_utils::*;
pub(crate) use file_type_ext::ImplFileTypeExt;
pub(crate) use hard_link_unchecked::hard_link_unchecked;
#[allow(unused_imports)]
pub(crate) use is_same_file::{is_different_file, is_different_file_metadata, is_same_file};
pub(crate) use metadata_ext::ImplMetadataExt;
pub(crate) use open_options_ext::ImplOpenOptionsExt;
pub(crate) use open_unchecked::open_unchecked;
pub(crate) use read_dir_inner::ReadDirInner;
pub(crate) use read_link_unchecked::read_link_unchecked;
pub(crate) use remove_dir_unchecked::remove_dir_unchecked;
pub(crate) use remove_file_unchecked::remove_file_unchecked;
pub(crate) use rename_unchecked::rename_unchecked;
pub(crate) use stat_unchecked::stat_unchecked;
pub(crate) use symlink_unchecked::symlink_unchecked;
pub(crate) use times::to_timespec;

// On Linux, there is a limit of 40 symlink expansions.
// Source: <https://man7.org/linux/man-pages/man7/path_resolution.7.html>
pub(crate) const MAX_SYMLINK_EXPANSIONS: u8 = 40;

pub(super) use oflags::*;

pub(crate) fn read_link(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    let start = MaybeOwnedFile::borrowed(start);
    let (dir, basename) = open_parent(start, path)?;
    read_link_unchecked(&dir, basename.as_ref(), PathBuf::new())
}

pub(crate) fn set_times_nofollow(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    let times = rustix::fs::Timestamps {
        last_access: to_timespec(atime)?,
        last_modification: to_timespec(mtime)?,
    };

    #[cfg(target_os = "freebsd")]
    if set_times_nofollow_fast(start, path, &times)? {
        return Ok(());
    }

    let start = MaybeOwnedFile::borrowed(start);
    let (dir, basename) = open_parent(start, path)?;

    Ok(utimensat(
        &*dir,
        basename,
        &times,
        AtFlags::SYMLINK_NOFOLLOW,
    )?)
}

pub(crate) fn set_times(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    #[cfg(target_os = "freebsd")]
    if set_times_fast(start, path, atime, mtime)? {
        return Ok(());
    }

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
        Err(err) => match Errno::from_io_error(&err) {
            Some(Errno::ACCESS) | Some(Errno::ISDIR) => (),
            _ => return Err(err),
        },
    }

    // Next try read.
    match open(start, path, OpenOptions::new().read(true)) {
        Ok(file) => return file.set_times(times),
        Err(err) => match Errno::from_io_error(&err) {
            Some(Errno::ACCESS) => (),
            _ => return Err(err),
        },
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    if set_times_fallback(start, path, atime, mtime)? {
        return Ok(());
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
