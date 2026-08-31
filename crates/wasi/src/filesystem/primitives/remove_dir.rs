//! This defines `remove_dir`, the primary entrypoint to sandboxed file
//! removal.

use crate::filesystem::primitives::remove_dir_impl;
use std::path::Path;
use std::{fs, io};

/// Perform a `rmdirat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
#[inline]
pub fn remove_dir(start: &fs::File, path: &Path) -> io::Result<()> {
    // Call the underlying implementation.
    remove_dir_impl(start, path)
}
