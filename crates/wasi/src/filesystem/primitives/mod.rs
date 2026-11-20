//! Filesystem utilities.

#![allow(
    clippy::unnecessary_fallible_conversions,
    reason = "typedefs are platform-specific"
)]
#![allow(trivial_numeric_casts, reason = "typedefs are platform-specific")]
#![allow(clippy::useless_conversion, reason = "typedefs are platform-specific")]

use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;
use std::{fs, io};

mod maybe_owned_file;
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

use maybe_owned_file::MaybeOwnedFile;
use open_parent::open_parent;
use open_unchecked_error::*;
use sys::*;

pub(crate) use open_options::*;
pub(crate) use sys::advise;
pub(crate) use sys::append_cursor_unspecified;
pub(crate) use sys::open_ambient_dir;
pub(crate) use sys::read_at_cursor_unspecified;
pub(crate) use sys::read_dir;
pub(crate) use sys::remove_file_or_symlink;
pub(crate) use sys::set_times;
pub(crate) use sys::set_times_nofollow;
pub(crate) use sys::status_flags;
pub(crate) use sys::write_at_cursor_unspecified;
pub(crate) use sys::{FileType, Metadata};

/// Access-pattern advice for a region of a file, as passed to
/// `posix_fadvise` on platforms that support it.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Advice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
    NoReuse,
}

bitflags::bitflags! {
    /// Descriptor flags.
    ///
    /// Note: This was called `fdflags` in earlier versions of WASI.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub(crate) struct DescriptorFlags: usize {
        /// Read mode: Data can be read.
        const READ = 0b1;
        /// Write mode: Data can be written to.
        const WRITE = 0b10;
        /// Request that writes be performed according to synchronized I/O file
        /// integrity completion. The data stored in the file and the file's
        /// metadata are synchronized. This is similar to `O_SYNC` in POSIX.
        ///
        /// The precise semantics of this operation have not yet been defined for
        /// WASI. At this time, it should be interpreted as a request, and not a
        /// requirement.
        const FILE_INTEGRITY_SYNC = 0b100;
        /// Request that writes be performed according to synchronized I/O data
        /// integrity completion. Only the data stored in the file is
        /// synchronized. This is similar to `O_DSYNC` in POSIX.
        ///
        /// The precise semantics of this operation have not yet been defined for
        /// WASI. At this time, it should be interpreted as a request, and not a
        /// requirement.
        const DATA_INTEGRITY_SYNC = 0b1000;
        /// Requests that reads be performed at the same level of integrity
        /// requested for writes. This is similar to `O_RSYNC` in POSIX.
        ///
        /// The precise semantics of this operation have not yet been defined for
        /// WASI. At this time, it should be interpreted as a request, and not a
        /// requirement.
        const REQUESTED_WRITE_SYNC = 0b10000;
        /// Mutating directories mode: Directory contents may be mutated.
        ///
        /// When this flag is unset on a descriptor, operations using the
        /// descriptor which would create, rename, delete, modify the data or
        /// metadata of filesystem objects, or obtain another handle which
        /// would permit any of those, shall fail with `error-code::read-only` if
        /// they would otherwise succeed.
        ///
        /// This may only be set on directories.
        const MUTATE_DIRECTORY = 0b100000;
    }
}

/// Set the timestamps of the file or directory that `fd` itself refers to.
pub(crate) fn set_times_on_fd(
    fd: &fs::File,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if sys::set_times_fast(fd, atime, mtime)? {
        return Ok(());
    }

    let mut times = fs::FileTimes::new();
    if let Some(atime) = atime {
        times = times.set_accessed(atime);
    }
    if let Some(mtime) = mtime {
        times = times.set_modified(mtime);
    }
    fd.set_times(times)
}

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
///
/// This is implemented by `open`ing up the parent component of the path and
/// then calling `symlink_unchecked` on the last component.
pub(crate) fn symlink(old_path: &Path, new_start: &fs::File, new_path: &Path) -> io::Result<()> {
    // Don't allow creating symlinks to absolute paths. This isn't strictly
    // necessary to preserve the sandbox, since `open` will refuse to follow
    // absolute symlinks in any case. However, it is useful to enforce this
    // restriction so that a WASI program can't trick some other non-WASI
    // program into following an absolute path.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    // Like `symlinkat`, an empty target is an error on all platforms.
    if old_path.as_os_str().is_empty() {
        return Err(errors::no_such_file_or_directory());
    }

    // Some platforms (Windows) need to know whether the target is a directory
    // when creating a symlink. The target is resolved relative to the
    // directory containing the new link, and the lookup is sandboxed to
    // `new_start` so nothing is learned about paths outside of it. Targets
    // which can't be looked up, such as dangling symlinks, are not
    // considered directories.
    let target_is_dir = || {
        let new_dir = new_path.parent().unwrap_or(Path::new(""));
        stat(new_start, &new_dir.join(old_path), FollowSymlinks::Yes).is_ok_and(|m| m.is_dir())
    };

    let (new_dir, new_basename) = open_parent(MaybeOwnedFile::borrowed(new_start), new_path)?;
    symlink_unchecked(old_path, &new_dir, new_basename.as_ref(), target_is_dir)
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
    // And remember whether either path ended with a slash so that we can still
    // fail if the source isn't a directory.
    let requires_dir = path_has_trailing_slash(old_path) || path_has_trailing_slash(new_path);
    let old_path = strip_dir_suffix(old_path);
    let new_path = strip_dir_suffix(new_path);

    let (old_dir, old_basename) = open_parent(old_start, &old_path)?;
    let (new_dir, new_basename) = open_parent(new_start, &new_path)?;

    // A trailing slash on either path requires the source to be a directory.
    //
    // On Linux, re-append a slash to the source and let the kernel check that
    // atomically with the rename. Linux never follows a symlink named with a
    // trailing slash in `renameat` and fails with `ENOTDIR` instead.
    #[cfg(target_os = "linux")]
    let mut concat;
    #[cfg(target_os = "linux")]
    let old_basename = if requires_dir {
        concat = old_basename.to_owned();
        concat.push("/");
        concat.as_os_str()
    } else {
        old_basename
    };

    // Elsewhere, such as on macOS and FreeBSD, `renameat` follows a symlink
    // named with a trailing slash. Check without following symlinks and rename
    // by the plain names instead, so such a symlink is never followed out of
    // `old_start`. The check isn't atomic with the rename, but whatever
    // replaces the source in between is still renamed by its plain name.
    #[cfg(not(target_os = "linux"))]
    if requires_dir
        && !stat_unchecked(&old_dir, Path::new(old_basename), FollowSymlinks::No)?.is_dir()
    {
        return Err(errors::is_not_directory());
    }

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
    // FreeBSD's kernel removes a symlink's target for `link/`, so leave
    // trailing slashes to the checks below.
    #[cfg(target_os = "freebsd")]
    if !path_has_trailing_slash(path) && sys::remove_dir_fast(start, path)? {
        return Ok(());
    }

    // As with `create_dir`, a trailing slash after a directory names the
    // directory itself rather than acting like a trailing slash-dot, so remove
    // it by its plain name. Other paths with a trailing slash still resolve
    // through `open_parent` below.
    if path_has_trailing_slash(path) {
        let stripped = strip_dir_suffix(path);
        if let Ok((dir, basename)) = open_parent(MaybeOwnedFile::borrowed(start), &stripped) {
            let basename = Path::new(basename);
            match stat_unchecked(&dir, basename, FollowSymlinks::No) {
                Ok(m) if m.is_dir() => return remove_dir_unchecked(&dir, basename),
                // Like Linux, fail on a symlink without following it, so its
                // target is never touched.
                Ok(m) if m.file_type().is_symlink() => return Err(errors::is_not_directory()),
                _ => {}
            }
        }
    }

    let start = MaybeOwnedFile::borrowed(start);
    let (dir, basename) = open_parent(start, path)?;
    remove_dir_unchecked(&dir, basename.as_ref())
}

pub(crate) fn stat(start: &fs::File, path: &Path, follow: FollowSymlinks) -> io::Result<Metadata> {
    // Optimization: if path has exactly one component and it's not ".." or
    // anything non-normal and we're not following symlinks we can go straight
    // to `stat_unchecked`, which can be faster than various paths below.
    if follow == FollowSymlinks::No && !path_requires_dir(path) {
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
