//! This defines `rename`, the primary entrypoint to sandboxed renaming.

use crate::filesystem::primitives::rename_impl;
use std::path::Path;
use std::{fs, io};

/// Perform a `renameat`-like operation, ensuring that the resolution of both
/// the old and new paths never escape the directory tree rooted at their
/// respective starts.
#[inline]
pub fn rename(
    old_start: &fs::File,
    old_path: &Path,
    new_start: &fs::File,
    new_path: &Path,
) -> io::Result<()> {
    // Call the underlying implementation.
    rename_impl(old_start, old_path, new_start, new_path)
}
