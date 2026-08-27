use crate::filesystem::primitives::{DirEntry, ReadDirInner};
use std::{fmt, fs, io};

/// Like `read_dir` but operates on the base directory itself, rather than
/// on a path based on it.
#[inline]
pub fn read_base_dir(start: &fs::File) -> io::Result<ReadDir> {
    Ok(ReadDir {
        inner: ReadDirInner::read_base_dir(start)?,
    })
}

/// Iterator over the entries in a directory.
///
/// This corresponds to [`std::fs::ReadDir`].
///
/// There is no `from_std` method, as `std::fs::ReadDir` doesn't provide a way
/// to construct a `ReadDir` without opening directories by ambient paths.
pub struct ReadDir {
    pub(crate) inner: ReadDirInner,
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|inner| inner.map(|inner| DirEntry { inner }))
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
