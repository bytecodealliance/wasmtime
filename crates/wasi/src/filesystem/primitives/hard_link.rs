//! This defines `hard_link`, the primary entrypoint to sandboxed hard-link
//! creation.

use crate::filesystem::primitives::hard_link_impl;
use std::path::Path;
use std::{fs, io};

/// Perform a `linkat`-like operation, ensuring that the resolution of the path
/// never escapes the directory tree rooted at `start`.
#[inline]
pub fn hard_link(
    old_start: &fs::File,
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    // Call the underlying implementation.
    hard_link_impl(old_start, old_path, new_start, new_path)
}
