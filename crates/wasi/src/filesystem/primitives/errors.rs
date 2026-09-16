use std::io;

#[cfg(not(windows))]
pub(crate) use crate::filesystem::primitives::unix::errors::*;
#[cfg(windows)]
pub(crate) use crate::filesystem::primitives::windows::errors::*;

#[cold]
pub(crate) fn escape_attempt() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "a path led outside of the filesystem",
    )
}
