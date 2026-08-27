use crate::filesystem::primitives::{Metadata, ReadDirInner};
use rustix::fs::DirEntry;
use std::ffi::{OsStr, OsString};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "wasi")]
use std::os::wasi::ffi::OsStrExt;
use std::{fmt, io};

pub(crate) struct DirEntryInner {
    pub(super) rustix: DirEntry,
    pub(super) read_dir: ReadDirInner,
}

impl DirEntryInner {
    #[inline]
    pub(crate) fn metadata(&self) -> io::Result<Metadata> {
        self.read_dir.metadata(self.file_name_bytes())
    }

    #[inline]
    pub(crate) fn file_name(&self) -> OsString {
        self.file_name_bytes().to_os_string()
    }

    #[inline]
    pub(crate) fn ino(&self) -> u64 {
        self.rustix.ino()
    }

    fn file_name_bytes(&self) -> &OsStr {
        OsStr::from_bytes(self.rustix.file_name().to_bytes())
    }
}

impl fmt::Debug for DirEntryInner {
    // Like libstd's version, but doesn't print the path.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DirEntry").field(&self.file_name()).finish()
    }
}
