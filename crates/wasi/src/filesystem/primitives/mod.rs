//! Filesystem utilities.

#![allow(
    trivial_numeric_casts,
    reason = "preexisting from when cap-primitives was imported"
)]
#![allow(
    unsafe_op_in_unsafe_fn,
    reason = "preexisting from when cap-primitives was imported"
)]
#![allow(
    clippy::unnecessary_fallible_conversions,
    reason = "platform-agnostic code can't always take advantage of this"
)]
#![allow(
    clippy::allow_attributes_without_reason,
    reason = "preexisting from when cap-primitives was imported"
)]

use std::path::{Path, PathBuf};
use std::{fs, io};

mod dir_entry;
mod dir_options;
mod file_type;
mod maybe_owned_file;
mod metadata;
mod open_options;
mod open_unchecked_error;
mod read_dir;

mod errors;
mod manually;
mod via_parent;

#[cfg(test)]
mod tests;

#[cfg(not(windows))]
mod rustix;
#[cfg(not(windows))]
use self::rustix::fs as sys;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::fs as sys;

#[cfg(windows)]
use file_type::_WindowsFileTypeExt;
use maybe_owned_file::MaybeOwnedFile;
use open_unchecked_error::*;
use sys::read_link_impl as read_link_contents;
use sys::*;

pub(crate) use dir_entry::DirEntry;
pub(crate) use dir_options::DirOptions;
pub(crate) use file_type::FileType;
#[cfg(any(unix, target_os = "vxworks"))]
pub(crate) use file_type::FileTypeExt;
#[cfg(windows)]
pub(crate) use metadata::_WindowsByHandle;
pub(crate) use metadata::{Metadata, MetadataExt};
pub(crate) use open_options::*;
pub(crate) use read_dir::read_base_dir;
pub(crate) use sys::create_dir_impl as create_dir;
pub(crate) use sys::hard_link_impl as hard_link;
pub(crate) use sys::open_ambient_dir_impl as open_ambient_dir;
pub(crate) use sys::open_impl as open;
pub(crate) use sys::remove_dir_impl as remove_dir;
pub(crate) use sys::remove_file_impl as remove_file;
pub(crate) use sys::rename_impl as rename;
pub(crate) use sys::set_times_impl as set_times;
pub(crate) use sys::set_times_nofollow_impl as set_times_nofollow;
pub(crate) use sys::stat_impl as stat;

/// Should symlinks be followed in the last component of a path?
///
/// This doesn't affect path components other than the last. So for example in
/// "foo/bar/baz", if "foo" or "bar" are symlinks, they will always be
/// followed. This enum value only determines whether "baz" is followed.
///
/// Instead of passing bare `bool`s as parameters, pass a distinct enum so that
/// the intent is clear.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum FollowSymlinks {
    /// Yes, do follow symlinks in the last component of a path.
    Yes,

    /// No, do not follow symlinks in the last component of a path.
    No,
}

/// Like [`read_link_contents`], but additionally verifies that the link target
/// is not absolute.
#[inline]
pub(crate) fn read_link(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    let contents = read_link_contents(start, path)?;

    // Don't allow reading symlinks to absolute paths. This isn't strictly
    // necessary to preserve the sandbox, since `open` will refuse to follow
    // absolute paths in any case. However, it is useful to enforce this
    // restriction to avoid leaking information about the host filesystem
    // outside the sandbox.
    if contents.has_root() {
        return Err(errors::escape_attempt());
    }

    Ok(contents)
}

/// Perform a `symlinkat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`. An error is
/// returned if the target path is absolute.
#[cfg(not(windows))]
#[inline]
pub(crate) fn symlink(old_path: &Path, new_start: &fs::File, new_path: &Path) -> io::Result<()> {
    // Don't allow creating symlinks to absolute paths. This isn't strictly
    // necessary to preserve the sandbox, since `open` will refuse to follow
    // absolute symlinks in any case. However, it is useful to enforce this
    // restriction so that a WASI program can't trick some other non-WASI
    // program into following an absolute path.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    sys::symlink_impl(old_path, new_start, new_path)
}

/// Perform a `symlink_file`-like operation, ensuring that the resolution of
/// the path never escapes the directory tree rooted at `start`.
#[cfg(windows)]
#[inline]
pub(crate) fn symlink_file(
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    // As above, don't allow creating symlinks to absolute paths.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    sys::symlink_file_impl(old_path, new_start, new_path)
}

/// Perform a `symlink_dir`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
#[cfg(windows)]
#[inline]
pub(crate) fn symlink_dir(
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    // As above, don't allow creating symlinks to absolute paths.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    sys::symlink_dir_impl(old_path, new_start, new_path)
}

/// Open a directory by performing an `openat`-like operation, ensuring that
/// the resolution of the path never escapes the directory tree rooted at
/// `start`.
#[inline]
fn open_dir(start: &fs::File, path: &Path) -> io::Result<fs::File> {
    open(start, path, &dir_options())
}

/// Open a directory by performing an unsandboxed `openat`-like operation.
#[inline]
#[allow(dead_code)]
fn open_dir_unchecked(start: &fs::File, path: &Path) -> io::Result<fs::File> {
    open_unchecked(start, path, &dir_options()).map_err(Into::into)
}

/// Like `open_dir_unchecked`, but additionally request the ability to read the
/// directory entries.
#[inline]
#[allow(dead_code)]
fn open_dir_for_reading_unchecked(
    start: &fs::File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<fs::File> {
    open_unchecked(start, path, readdir_options().follow(follow)).map_err(Into::into)
}
