use crate::filesystem::primitives::{
    Advice, DescriptorFlags, FollowSymlinks, MaybeOwnedFile, OpenOptions, open, open_parent,
};
use rustix::fs::{AtFlags, Dir, OFlags, fcntl_getfl, fcntl_setfl, utimensat};
use rustix::io::Errno;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::FileExt;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

mod create_dir_unchecked;
mod dir_utils;
mod hard_link_unchecked;
mod metadata;
mod oflags;
mod open_options_ext;
mod open_unchecked;
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

pub(crate) use super::remove_file as remove_file_or_symlink;
pub(crate) use create_dir_unchecked::create_dir_unchecked;
pub(crate) use dir_utils::*;
pub(crate) use hard_link_unchecked::hard_link_unchecked;
pub(crate) use metadata::Metadata;
pub(crate) use open_options_ext::ImplOpenOptionsExt;
pub(crate) use open_unchecked::open_unchecked;
pub(crate) use read_link_unchecked::read_link_unchecked;
pub(crate) use remove_dir_unchecked::remove_dir_unchecked;
pub(crate) use remove_file_unchecked::remove_file_unchecked;
pub(crate) use rename_unchecked::rename_unchecked;
pub(crate) use rustix::fs::FileType;
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

pub(crate) fn read_dir(
    file: &fs::File,
) -> io::Result<impl Iterator<Item = io::Result<(OsString, FileType)>> + 'static> {
    // Open ".", to obtain a new independent file descriptor. Don't use
    // `dup` since in that case the resulting file descriptor would share
    // a current position with the original, and `read_dir` calls after
    // the first `read_dir` call wouldn't start from the beginning.
    let fd = open_unchecked(
        file,
        Component::CurDir.as_ref(),
        readdir_options().follow(FollowSymlinks::No),
    )?;
    let mut dir = Dir::new(fd)?;
    Ok(std::iter::from_fn(move || {
        let result = (|| {
            loop {
                let Some(entry) = dir.read() else {
                    return Ok(None);
                };
                let entry = entry?;
                let file_name = entry.file_name().to_bytes();
                if file_name == Component::CurDir.as_os_str().as_bytes()
                    || file_name == Component::ParentDir.as_os_str().as_bytes()
                {
                    continue;
                }

                let file_type = cfg_select! {
                    target_os = "illumos" => FileType::Unknown,
                    _ => entry.file_type(),

                };

                return Ok(Some((OsString::from_vec(file_name.to_vec()), file_type)));
            }
        })();
        match result {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }))
}

/// Queries the synchronization-related status flags of `file`, as set at
/// open time or with `F_SETFL`.
pub(crate) fn status_flags(file: &fs::File) -> io::Result<DescriptorFlags> {
    let flags = fcntl_getfl(file)?;
    let mut ret = DescriptorFlags::empty();
    #[cfg(not(target_os = "redox"))]
    ret.set(
        DescriptorFlags::REQUESTED_WRITE_SYNC,
        flags.contains(OFlags::DSYNC),
    );
    ret.set(
        DescriptorFlags::FILE_INTEGRITY_SYNC,
        flags.contains(OFlags::SYNC),
    );
    #[cfg(not(any(target_vendor = "apple", target_os = "freebsd", target_os = "redox")))]
    ret.set(
        DescriptorFlags::DATA_INTEGRITY_SYNC,
        flags.contains(OFlags::RSYNC),
    );
    Ok(ret)
}

/// Provides the OS with `advice` about the expected access pattern of the
/// `len` bytes of `file` starting at `offset`.
pub(crate) fn advise(file: &fs::File, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
    cfg_select! {
        target_vendor = "apple" => {
            match advice {
                Advice::WillNeed => {
                    rustix::fs::fcntl_rdadvise(file, offset, len)?;
                }
                Advice::Normal |
                Advice::Sequential |
                Advice::Random |
                Advice::DontNeed |
                Advice::NoReuse => {}
            }
        }
        any(target_os = "linux", target_os = "android") => {
            use std::num::NonZeroU64;
            let advice = match advice {
                Advice::Normal => rustix::fs::Advice::Normal,
                Advice::Sequential => rustix::fs::Advice::Sequential,
                Advice::Random => rustix::fs::Advice::Random,
                Advice::WillNeed => rustix::fs::Advice::WillNeed,
                Advice::DontNeed => rustix::fs::Advice::DontNeed,
                Advice::NoReuse => rustix::fs::Advice::NoReuse,
            };
            rustix::fs::fadvise(file, offset, NonZeroU64::new(len), advice)?;
        }
        _ => {
            // noop on other platforms
            let _ = (file, offset, len, advice);
        }
    }
    Ok(())
}

/// Appends `data` to the end of `file`, leaving the file's current position
/// unspecified afterwards.
pub(crate) fn append_cursor_unspecified(file: &fs::File, data: &[u8]) -> io::Result<usize> {
    // On Linux, use `pwritev2`.
    #[cfg(target_os = "linux")]
    {
        use rustix::io::{ReadWriteFlags, pwritev2};
        use std::io::IoSlice;

        let iovs = [IoSlice::new(data)];
        match pwritev2(&file, &iovs, 0, ReadWriteFlags::APPEND) {
            Err(Errno::NOSYS) | Err(Errno::NOTSUP) => {}
            otherwise => return Ok(otherwise?),
        }
    }

    // Otherwise use `F_SETFL` to switch the file description to append
    // mode, do the write, and switch back. This is not atomic with
    // respect to other users of the file description, but WASI isn't fully
    // threaded right now anyway.
    let old_flags = fcntl_getfl(&file)?;
    fcntl_setfl(&file, old_flags | OFlags::APPEND)?;
    let result = rustix::io::write(&file, data);
    fcntl_setfl(&file, old_flags).unwrap();
    Ok(result?)
}

/// Writes `data` to `file` at `pos`, leaving the file's current position
/// unspecified afterwards.
pub(crate) fn write_at_cursor_unspecified(
    file: &fs::File,
    data: &[u8],
    pos: u64,
) -> io::Result<usize> {
    file.write_at(data, pos)
}

/// Reads from `file` at `pos` into `buf`, leaving the file's current
/// position unspecified afterwards.
pub(crate) fn read_at_cursor_unspecified(
    file: &fs::File,
    buf: &mut [u8],
    pos: u64,
) -> io::Result<usize> {
    file.read_at(buf, pos)
}
