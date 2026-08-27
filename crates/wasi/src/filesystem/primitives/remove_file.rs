//! This defines `remove_file`, the primary entrypoint to sandboxed file
//! removal.

use crate::filesystem::primitives::remove_file_impl;
use std::path::Path;
use std::{fs, io};

/// Perform a `remove_fileat`-like operation, ensuring that the resolution of
/// the path never escapes the directory tree rooted at `start`.
#[inline]
pub fn remove_file(start: &fs::File, path: &Path) -> io::Result<()> {
    // Call the underlying implementation.
    remove_file_impl(start, path)
}
