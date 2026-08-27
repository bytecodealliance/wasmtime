//! Access test functions.

use crate::filesystem::primitives::{FollowSymlinks, access_impl};
use std::path::Path;
use std::{fs, io};

/// Access modes for use with [`DirExt::access`].
#[derive(Clone, Copy, Debug)]
pub struct AccessModes {
    /// Is the object readable?
    pub readable: bool,
    /// Is the object writable?
    pub writable: bool,
    /// Is the object executable?
    pub executable: bool,
}

/// Access modes for use with [`DirExt::access`].
#[derive(Clone, Copy, Debug)]
pub enum AccessType {
    /// Test whether the named object is accessible in the given modes.
    Access(AccessModes),

    /// Test whether the named object exists.
    Exists,
}

/// Canonicalize the given path, ensuring that the resolution of the path never
/// escapes the directory tree rooted at `start`.
pub fn access(
    start: &fs::File,
    path: &Path,
    type_: AccessType,
    follow: FollowSymlinks,
) -> io::Result<()> {
    // Call the underlying implementation.
    access_impl(start, path, type_, follow)
}
