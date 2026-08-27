//! This defines `create_dir`, the primary entrypoint to sandboxed directory
//! creation.

use crate::filesystem::primitives::{DirOptions, create_dir_impl};
use std::path::Path;
use std::{fs, io};

/// Perform a `mkdirat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
#[inline]
pub fn create_dir(start: &fs::File, path: &Path, options: &DirOptions) -> io::Result<()> {
    // Call the underlying implementation.
    create_dir_impl(start, path, options)
}
