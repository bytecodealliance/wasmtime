//! This defines `symlink`, the primary entrypoint to sandboxed symlink
//! creation.

use crate::filesystem::primitives::errors;
use std::path::Path;
use std::{fs, io};

/// Perform a `symlinkat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`. An error is
/// returned if the target path is absolute.
#[cfg(not(windows))]
#[inline]
pub fn symlink(old_path: &Path, new_start: &fs::File, new_path: &Path) -> io::Result<()> {
    // Don't allow creating symlinks to absolute paths. This isn't strictly
    // necessary to preserve the sandbox, since `open` will refuse to follow
    // absolute symlinks in any case. However, it is useful to enforce this
    // restriction so that a WASI program can't trick some other non-WASI
    // program into following an absolute path.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    write_symlink_impl(old_path, new_start, new_path)
}

#[cfg(not(windows))]
fn write_symlink_impl(old_path: &Path, new_start: &fs::File, new_path: &Path) -> io::Result<()> {
    use crate::filesystem::primitives::symlink_impl;

    // Call the underlying implementation.
    symlink_impl(old_path, new_start, new_path)
}

/// Perform a `symlink_file`-like operation, ensuring that the resolution of
/// the path never escapes the directory tree rooted at `start`.
#[cfg(windows)]
#[inline]
pub fn symlink_file(old_path: &Path, new_start: &fs::File, new_path: &Path) -> io::Result<()> {
    use crate::filesystem::primitives::symlink_file_impl;

    // As above, don't allow creating symlinks to absolute paths.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    // Call the underlying implementation.
    symlink_file_impl(old_path, new_start, new_path)
}

/// Perform a `symlink_dir`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
#[cfg(windows)]
#[inline]
pub fn symlink_dir(old_path: &Path, new_start: &fs::File, new_path: &Path) -> io::Result<()> {
    use crate::filesystem::primitives::symlink_dir_impl;

    // As above, don't allow creating symlinks to absolute paths.
    if old_path.has_root() {
        return Err(errors::escape_attempt());
    }

    // Call the underlying implementation.
    symlink_dir_impl(old_path, new_start, new_path)
}
