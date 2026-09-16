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

use std::path::{Component, Path, PathBuf};
use std::{fs, io};

mod file_type;
mod maybe_owned_file;
mod metadata;
mod open_options;
mod open_parent;
mod open_unchecked_error;

mod errors;
mod manually;

#[cfg(test)]
mod tests;

#[cfg(not(windows))]
mod unix;
#[cfg(not(windows))]
use self::unix as sys;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as sys;

#[cfg(windows)]
use file_type::_WindowsFileTypeExt;
use maybe_owned_file::MaybeOwnedFile;
use open_parent::open_parent;
use open_unchecked_error::*;
use sys::*;

pub(crate) use file_type::FileType;
#[cfg(any(unix, target_os = "vxworks"))]
pub(crate) use file_type::FileTypeExt;
#[cfg(windows)]
pub(crate) use metadata::_WindowsByHandle;
pub(crate) use metadata::{Metadata, MetadataExt};
pub(crate) use open_options::*;
pub(crate) use sys::open_ambient_dir;
pub(crate) use sys::read_dir;
pub(crate) use sys::set_times;
pub(crate) use sys::set_times_nofollow;

pub(crate) fn open(start: &fs::File, path: &Path, options: &OpenOptions) -> io::Result<fs::File> {
    #[cfg(any(
        windows,
        target_os = "freebsd",
        target_os = "android",
        target_os = "linux",
    ))]
    if let Some(file) = sys::open_fast(start, path, options)? {
        return Ok(file);
    }
    manually::open(start, path, options)
}

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

/// Perform a `readlinkat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`, and additionally
/// verifying that the link target is not absolute.
pub(crate) fn read_link(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    // On most platforms `read_link` is implemented by `open`ing up the parent
    // component of the path and then calling `read_link_unchecked` on the last
    // component.
    //
    // This technique doesn't work in all cases on Windows. In particular, a
    // directory symlink such as `C:\Documents and Settings` may not grant any
    // access other than what is needed to resolve the symlink, so
    // `open_parent`'s technique of returning a relative path of `.` from that
    // point doesn't work, because opening `.` within such a directory is
    // denied. Consequently, we use a different implementation on Windows.
    let contents = sys::read_link(start, path)?;

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

    let new_start = MaybeOwnedFile::borrowed(new_start);
    let (new_dir, new_basename) = open_parent(new_start, new_path)?;
    symlink_unchecked(old_path, &new_dir, new_basename.as_ref())
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

    let new_start = MaybeOwnedFile::borrowed(new_start);
    let (new_dir, new_basename) = open_parent(new_start, new_path)?;
    symlink_file_unchecked(old_path, &new_dir, new_basename.as_ref())
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

    let new_start = MaybeOwnedFile::borrowed(new_start);
    let (new_dir, new_basename) = open_parent(new_start, new_path)?;
    symlink_dir_unchecked(old_path, &new_dir, new_basename.as_ref())
}

/// Perform a `mkdirat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
///
/// This is implemented by `open`ing up the parent component of the path and
/// then calling `create_dir_unchecked` on the last component.
pub(crate) fn create_dir(start: &fs::File, path: &Path) -> io::Result<()> {
    let start = MaybeOwnedFile::borrowed(start);

    // As a special case, `create_dir` ignores a trailing slash rather than
    // treating it as equivalent to a trailing slash-dot, so strip any trailing
    // slashes.
    let path = strip_dir_suffix(path);

    let (dir, basename) = open_parent(start, &path)?;

    create_dir_unchecked(&dir, basename.as_ref())
}

/// Perform a `linkat`-like operation, ensuring that the resolution of both
/// paths never escapes the directory tree rooted at their respective starts.
///
/// This is implemented by `open`ing up the parent component of each path and
/// then calling `hard_link_unchecked` on the last components.
pub(crate) fn hard_link(
    old_start: &fs::File,
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    let old_start = MaybeOwnedFile::borrowed(old_start);
    let new_start = MaybeOwnedFile::borrowed(new_start);

    let (old_dir, old_basename) = open_parent(old_start, old_path)?;
    let (new_dir, new_basename) = open_parent(new_start, new_path)?;

    hard_link_unchecked(
        &old_dir,
        old_basename.as_ref(),
        &new_dir,
        new_basename.as_ref(),
    )
}

/// Perform a `renameat`-like operation, ensuring that the resolution of both
/// paths never escapes the directory tree rooted at their respective starts.
///
/// This is implemented by `open`ing up the parent component of each path and
/// then calling `rename_unchecked` on the last components.
pub(crate) fn rename(
    old_start: &fs::File,
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    let old_start = MaybeOwnedFile::borrowed(old_start);
    let new_start = MaybeOwnedFile::borrowed(new_start);

    // As a special case, `rename` ignores a trailing slash rather than treating
    // it as equivalent to a trailing slash-dot, so strip any trailing slashes
    // for the purposes of `open_parent`.
    //
    // And on Unix, remember whether the source started with a slash so that we
    // can still fail if it is and the source is a regular file.
    #[cfg(unix)]
    let old_starts_with_slash = path_has_trailing_slash(old_path);
    let old_path = strip_dir_suffix(old_path);
    let new_path = strip_dir_suffix(new_path);

    let (old_dir, old_basename) = open_parent(old_start, &old_path)?;
    let (new_dir, new_basename) = open_parent(new_start, &new_path)?;

    // On Unix, re-append a slash if needed.
    #[cfg(unix)]
    let concat;
    #[cfg(unix)]
    let old_basename = if old_starts_with_slash {
        concat = append_dir_suffix(old_basename.to_owned().into());
        concat.as_os_str()
    } else {
        old_basename
    };

    rename_unchecked(
        &old_dir,
        old_basename.as_ref(),
        &new_dir,
        new_basename.as_ref(),
    )
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

pub(crate) fn remove_file(start: &fs::File, path: &Path) -> io::Result<()> {
    #[cfg(target_os = "freebsd")]
    if sys::remove_file_fast(start, path)? {
        return Ok(());
    }

    let start = MaybeOwnedFile::borrowed(start);
    let (dir, basename) = open_parent(start, path)?;
    remove_file_unchecked(&dir, basename.as_ref())
}

pub(crate) fn remove_dir(start: &fs::File, path: &Path) -> io::Result<()> {
    #[cfg(target_os = "freebsd")]
    if sys::remove_dir_fast(start, path)? {
        return Ok(());
    }

    let start = MaybeOwnedFile::borrowed(start);
    let (dir, basename) = open_parent(start, path)?;
    remove_dir_unchecked(&dir, basename.as_ref())
}

pub(crate) fn stat(start: &fs::File, path: &Path, follow: FollowSymlinks) -> io::Result<Metadata> {
    // Optimization: if path has exactly one component and it's not ".." or
    // anything non-normal and we're not following symlinks we can go straight
    // to `stat_unchecked`, which can be faster than various paths below.
    if follow == FollowSymlinks::No {
        let mut components = path.components();
        if let Some(Component::Normal(component)) = components.next() {
            if components.next().is_none() {
                return stat_unchecked(start, component.as_ref(), FollowSymlinks::No);
            }
        }
    }

    #[cfg(any(target_os = "freebsd", target_os = "android", target_os = "linux",))]
    if let Some(stat) = sys::stat_fast(start, path, follow)? {
        return Ok(stat);
    }

    manually::stat(start, path, follow)
}
