use crate::filesystem::primitives::unix::{compute_oflags, to_timespec};
use crate::filesystem::primitives::{
    FollowSymlinks, ImplMetadataExt, Metadata, OpenOptions, errors,
};
use rustix::fs::{
    AtFlags, CWD, Mode, OFlags, RawMode, Timestamps, openat, statat, unlinkat, utimensat,
};
use rustix::io::Errno;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::SystemTime;

static WORKING: AtomicBool = AtomicBool::new(false);
static CHECKED: AtomicBool = AtomicBool::new(false);

#[inline]
fn beneath_supported() -> bool {
    if WORKING.load(Relaxed) {
        return true;
    }
    if CHECKED.load(Relaxed) {
        return false;
    }
    check_beneath_supported()
}

#[cold]
fn check_beneath_supported() -> bool {
    // `RESOLVE_BENEATH` was introduced in FreeBSD 13, but opening `..` within
    // the root directory re-opened the root directory. In FreeBSD 14, it fails
    // as cap-std expects.
    if let Ok(root) = openat(CWD, c"/", OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty()) {
        // Unknown O_ flags get ignored but AT_ flags have strict checks, so we use that.
        if let Err(Errno::NOTCAPABLE) = statat(root, c"..", AtFlags::RESOLVE_BENEATH) {
            WORKING.store(true, Relaxed);
            return true;
        }
    }

    CHECKED.store(true, Relaxed);
    false
}

pub(crate) fn open_fast(
    start: &fs::File,
    path: &Path,
    options: &OpenOptions,
) -> io::Result<Option<fs::File>> {
    if !beneath_supported() {
        return Ok(None);
    }

    let oflags = compute_oflags(options)? | OFlags::RESOLVE_BENEATH;

    let mode = if oflags.contains(OFlags::CREATE) {
        Mode::from_bits((options.ext.mode & 0o7777) as RawMode).unwrap()
    } else {
        Mode::empty()
    };

    match openat(start, path, oflags, mode) {
        Ok(file) => Ok(Some(file.into())),
        Err(rustix::io::Errno::NOTCAPABLE) => Err(errors::escape_attempt()),
        Err(err) => Err(err.into()),
    }
}

pub(crate) fn remove_file_fast(start: &fs::File, path: &Path) -> io::Result<bool> {
    if !beneath_supported() {
        return Ok(false);
    }
    unlinkat(start, path, AtFlags::RESOLVE_BENEATH)?;
    Ok(true)
}

pub(crate) fn remove_dir_fast(start: &fs::File, path: &Path) -> io::Result<bool> {
    if !beneath_supported() {
        return Ok(false);
    }
    unlinkat(start, path, AtFlags::RESOLVE_BENEATH | AtFlags::REMOVEDIR)?;
    Ok(true)
}

pub(crate) fn set_times_nofollow_fast(
    start: &fs::File,
    path: &Path,
    times: &Timestamps,
) -> io::Result<bool> {
    if !beneath_supported() {
        return Ok(false);
    }
    utimensat(
        start,
        path,
        times,
        AtFlags::RESOLVE_BENEATH | AtFlags::SYMLINK_NOFOLLOW,
    )?;
    Ok(true)
}

pub(crate) fn stat_fast(
    start: &fs::File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<Option<Metadata>> {
    if !beneath_supported() {
        return Ok(None);
    }
    let flags = AtFlags::RESOLVE_BENEATH
        | if follow == FollowSymlinks::Yes {
            AtFlags::empty()
        } else {
            AtFlags::SYMLINK_NOFOLLOW
        };
    let stat = ImplMetadataExt::from_rustix(statat(start, path, flags)?);
    Ok(Some(stat))
}

pub(crate) fn set_times_fast(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<bool> {
    if !beneath_supported() {
        return Ok(false);
    }

    let times = Timestamps {
        last_access: to_timespec(atime)?,
        last_modification: to_timespec(mtime)?,
    };

    utimensat(start, path, &times, AtFlags::RESOLVE_BENEATH)?;
    Ok(true)
}
