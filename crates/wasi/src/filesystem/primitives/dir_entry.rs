use crate::filesystem::primitives::{DirEntryInner, Metadata};
#[cfg(not(windows))]
use rustix::fs::DirEntryExt;
use std::ffi::OsString;
use std::{fmt, io};

/// Entries returned by the `ReadDir` iterator.
///
/// This corresponds to [`std::fs::DirEntry`].
///
/// Unlike `std::fs::DirEntry`, this API has no `DirEntry::path`, because
/// absolute paths don't interoperate well with the capability model.
///
/// There is a `file_name` function, however there are also `open`,
/// `open_with`, `open_dir`, `remove_file`, and `remove_dir` functions for
/// opening or removing the entry directly, which can be more efficient and
/// convenient.
///
/// There is no `from_std` method, as `std::fs::DirEntry` doesn't provide a way
/// to construct a `DirEntry` without opening directories by ambient paths.
pub struct DirEntry {
    pub(crate) inner: DirEntryInner,
}

impl DirEntry {
    /// Returns the metadata for the file that this entry points at.
    ///
    /// This corresponds to [`std::fs::DirEntry::metadata`].
    ///
    /// # Platform-specific behavior
    ///
    /// On Windows, this produces a `Metadata` object which does not contain
    /// the optional values returned by [`MetadataExt`]. Use
    /// [`cap_fs_ext::DirEntryExt::full_metadata`] to obtain a `Metadata` with
    /// the values filled in.
    ///
    /// [`MetadataExt`]: https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
    /// [`cap_fs_ext::DirEntryExt::full_metadata`]: https://docs.rs/cap-fs-ext/latest/cap_fs_ext/trait.DirEntryExt.html#tymethod.full_metadata
    #[inline]
    pub fn metadata(&self) -> io::Result<Metadata> {
        self.inner.metadata()
    }

    /// Returns the bare file name of this directory entry without any other
    /// leading path component.
    ///
    /// This corresponds to [`std::fs::DirEntry::file_name`].
    #[inline]
    pub fn file_name(&self) -> OsString {
        self.inner.file_name()
    }
}

#[cfg(not(windows))]
impl DirEntryExt for DirEntry {
    #[inline]
    fn ino(&self) -> u64 {
        self.inner.ino()
    }
}

impl fmt::Debug for DirEntry {
    // Like libstd's version, but doesn't print the path.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
