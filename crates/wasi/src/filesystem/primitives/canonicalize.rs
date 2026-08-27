//! Sandboxed path canonicalization.

use crate::filesystem::primitives::canonicalize_impl;
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Canonicalize the given path, ensuring that the resolution of the path never
/// escapes the directory tree rooted at `start`.
#[inline]
pub fn canonicalize(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    // Call the underlying implementation.
    canonicalize_impl(start, path)
}
