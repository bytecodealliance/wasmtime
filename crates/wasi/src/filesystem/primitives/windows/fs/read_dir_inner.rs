use super::get_path::concatenate;
use crate::filesystem::primitives::DirEntryInner;
use std::path::{Component, Path};
use std::{fmt, fs, io};

pub(crate) struct ReadDirInner {
    std: fs::ReadDir,
}

impl ReadDirInner {
    pub(crate) fn read_base_dir(start: &fs::File) -> io::Result<Self> {
        Self::new_unchecked(&start, Component::CurDir.as_ref())
    }

    pub(crate) fn new_unchecked(start: &fs::File, path: &Path) -> io::Result<Self> {
        let full_path = concatenate(start, path)?;
        Ok(Self {
            std: fs::read_dir(full_path)?,
        })
    }
}

impl Iterator for ReadDirInner {
    type Item = io::Result<DirEntryInner>;

    fn next(&mut self) -> Option<Self::Item> {
        self.std
            .next()
            .map(|result| result.map(DirEntryInner::from_std))
    }
}

impl fmt::Debug for ReadDirInner {
    // Like libstd's version, but doesn't print the path.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("ReadDir");
        // `fs::ReadDir`'s `Debug` just prints the path, and since we're not
        // printing that, we don't have anything else to print.
        b.finish()
    }
}
