//! This defines `stat`, the primary entrypoint to sandboxed metadata querying.

use crate::filesystem::primitives::{FollowSymlinks, Metadata, stat_impl};
use std::path::Path;
use std::{fs, io};

/// Perform an `fstatat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
#[inline]
pub fn stat(start: &fs::File, path: &Path, follow: FollowSymlinks) -> io::Result<Metadata> {
    // Call the underlying implementation.
    stat_impl(start, path, follow)
}
