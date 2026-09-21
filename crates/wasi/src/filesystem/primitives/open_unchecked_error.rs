use std::io;

#[derive(Debug)]
pub(crate) enum OpenUncheckedError {
    Other(io::Error),
    Symlink(io::Error, SymlinkKind),
    NotFound(io::Error),
}

#[cfg(not(windows))]
pub(crate) type SymlinkKind = ();

#[cfg(windows)]
#[derive(Debug)]
pub(crate) enum SymlinkKind {
    File,
    Dir,
}

impl From<OpenUncheckedError> for io::Error {
    fn from(error: OpenUncheckedError) -> Self {
        match error {
            OpenUncheckedError::Other(err)
            | OpenUncheckedError::Symlink(err, _)
            | OpenUncheckedError::NotFound(err) => err,
        }
    }
}
