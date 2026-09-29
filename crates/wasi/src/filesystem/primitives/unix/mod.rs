use crate::filesystem::primitives::{
    FileType, FollowSymlinks, MaybeOwnedFile, Metadata, OpenOptions, open, open_parent,
};
use rustix::fs::{AtFlags, Dir, RawMode, utimensat};
use rustix::io::Errno;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

mod create_dir_unchecked;
mod dir_utils;
mod hard_link_unchecked;
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

pub(crate) use create_dir_unchecked::create_dir_unchecked;
pub(crate) use dir_utils::*;
pub(crate) use hard_link_unchecked::hard_link_unchecked;
pub(crate) use open_options_ext::ImplOpenOptionsExt;
pub(crate) use open_unchecked::open_unchecked;
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
                    target_os = "illumos" => rustix::fs::FileType::Unknown,
                    _ => entry.file_type(),

                };
                let file_type = FileType::Unix(file_type);

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

pub(super) fn system_time_from_rustix(sec: i64, nsec: u64) -> Option<SystemTime> {
    if sec >= 0 {
        SystemTime::UNIX_EPOCH.checked_add(Duration::new(u64::try_from(sec).unwrap(), nsec as _))
    } else {
        SystemTime::UNIX_EPOCH
            .checked_sub(Duration::new(sec.unsigned_abs(), 0))
            .map(|t| t.checked_add(Duration::new(0, nsec as u32)))
            .flatten()
    }
}

impl Metadata {
    #[inline]
    pub fn dev(&self) -> u64 {
        match self {
            Metadata::Std(m) => m.dev(),
            #[allow(unused_comparisons, reason = "platform-specific typedef")]
            Metadata::Stat(m) => {
                // The type of `st_dev` is `dev_t` which is signed on some
                // platforms and unsigned on other platforms. A `u64` is enough
                // to work for all unsigned platforms, and for signed platforms
                // perform a sign extension to `i64` and then view that as an
                // unsigned 64-bit number instead.
                //
                // Note that the `unused_comparisons` is ignored here for
                // platforms where it's unsigned since the first branch here
                // will never be taken.
                if m.st_dev < 0 {
                    i64::try_from(m.st_dev).unwrap().cast_unsigned()
                } else {
                    u64::try_from(m.st_dev).unwrap()
                }
            }
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => rustix::fs::makedev(m.stx_dev_major, m.stx_dev_minor),
        }
    }

    #[inline]
    pub fn ino(&self) -> u64 {
        match self {
            Metadata::Std(m) => m.ino(),
            Metadata::Stat(m) => m.st_ino,
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => m.stx_ino,
        }
    }

    #[inline]
    pub fn nlink(&self) -> u64 {
        match self {
            Metadata::Std(m) => m.nlink(),
            Metadata::Stat(m) => m.st_nlink.into(),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => m.stx_nlink.into(),
        }
    }
}

impl FileType {
    pub(super) fn from_raw_mode(raw_mode: RawMode) -> Self {
        Self::Unix(rustix::fs::FileType::from_raw_mode(raw_mode))
    }

    pub fn is_char_device(&self) -> bool {
        match self {
            Self::Std(std) => std.is_char_device(),
            #[cfg(unix)]
            Self::Unix(unix) => unix.is_char_device(),
        }
    }

    pub fn is_block_device(&self) -> bool {
        match self {
            Self::Std(std) => std.is_block_device(),
            #[cfg(unix)]
            Self::Unix(unix) => unix.is_block_device(),
        }
    }
}

/// It should be possible to represent times before the Epoch.
/// https://github.com/bytecodealliance/cap-std/issues/328
#[test]
fn negative_time() {
    let system_time = system_time_from_rustix(-1, 1).unwrap();
    let d = SystemTime::UNIX_EPOCH.duration_since(system_time).unwrap();
    assert_eq!(d.as_secs(), 0);
    assert_eq!(d.subsec_nanos(), 999999999);
}
