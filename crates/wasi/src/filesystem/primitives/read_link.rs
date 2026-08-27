//! This defines `read_link`, the primary entrypoint to sandboxed symbolic link
//! dereferencing.

use crate::filesystem::primitives::{errors, read_link_impl};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Perform a `readlinkat`-like operation, ensuring that the resolution of the
/// link path never escapes the directory tree rooted at `start`.
#[inline]
pub fn read_link_contents(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    // Call the underlying implementation.
    read_link_impl(start, path)
}

/// Perform a `readlinkat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`, and also verifies
/// that the link target is not absolute.
#[inline]
pub fn read_link(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    // Call the underlying implementation.
    let result = read_link_contents(start, path);

    // Don't allow reading symlinks to absolute paths. This isn't strictly
    // necessary to preserve the sandbox, since `open` will refuse to follow
    // absolute paths in any case. However, it is useful to enforce this
    // restriction to avoid leaking information about the host filesystem
    // outside the sandbox.
    if let Ok(path) = &result {
        if path.has_root() {
            return Err(errors::escape_attempt());
        }
    }

    result
}
