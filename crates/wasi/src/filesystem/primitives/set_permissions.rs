//! This defines `set_permissions`, the primary entrypoint to sandboxed
//! filesystem permissions modification.

use crate::filesystem::primitives::{
    Permissions, set_permissions_impl, set_symlink_permissions_impl,
};
use std::path::Path;
use std::{fs, io};

/// Perform a `chmodat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
#[inline]
pub fn set_permissions(start: &fs::File, path: &Path, perm: Permissions) -> io::Result<()> {
    // Call the underlying implementation.
    set_permissions_impl(start, path, perm)
}

/// Perform a `chmodat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`, without following
/// symlinks.
#[inline]
pub fn set_symlink_permissions(start: &fs::File, path: &Path, perm: Permissions) -> io::Result<()> {
    // Call the underlying implementation.
    set_symlink_permissions_impl(start, path, perm)
}
