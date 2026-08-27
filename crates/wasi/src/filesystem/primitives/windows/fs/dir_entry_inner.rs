use crate::filesystem::primitives::Metadata;
use std::ffi::OsString;
use std::{fmt, fs, io};

pub(crate) struct DirEntryInner {
    std: fs::DirEntry,
}

impl DirEntryInner {
    #[inline]
    pub(crate) fn metadata(&self) -> io::Result<Metadata> {
        self.std.metadata().map(Metadata::from_just_metadata)
    }

    #[inline]
    pub(crate) fn file_name(&self) -> OsString {
        self.std.file_name()
    }

    #[inline]
    pub(super) fn from_std(std: fs::DirEntry) -> Self {
        Self { std }
    }
}

impl fmt::Debug for DirEntryInner {
    // Like libstd's version, but doesn't print the path.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DirEntry").field(&self.file_name()).finish()
    }
}
