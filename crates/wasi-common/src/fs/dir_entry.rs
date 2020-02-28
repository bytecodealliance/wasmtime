use crate::fs::{FileType, Metadata};
use std::{ffi, io};

/// Entries returned by the ReadDir iterator.
///
/// This corresponds to [`std::fs::DirEntry`].
///
/// Unlike `std::fs::DirEntry`, this API has no `DirEntry::path`, because
/// absolute paths don't interoperate well with the capability-oriented
/// security model.
///
/// TODO: Not yet implemented.
///
/// [`std::fs::DirEntry`]: https://doc.rust-lang.org/std/fs/struct.DirEntry.html
pub struct DirEntry {}

impl DirEntry {
    /// Returns the metadata for the file that this entry points at.
    ///
    /// This corresponds to [`std::fs::DirEntry::metadata`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::DirEntry::metadata`]: https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.metadata
    pub fn metadata(&self) -> io::Result<Metadata> {
        unimplemented!("DirEntry::metadata");
    }

    /// Returns the file type for the file that this entry points at.
    ///
    /// This to [`std::fs::DirEntry::file_type`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::DirEntry::file_type`]: https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.file_type
    pub fn file_type(&self) -> io::Result<FileType> {
        unimplemented!("DirEntry::file_type");
    }

    /// Returns the bare file name of this directory entry without any other leading path component.
    ///
    /// This corresponds to [`std::fs::DirEntry::file_name`], though it returns
    /// `String` rather than `OsString`.
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::DirEntry::file_name`]: https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.file_name
    pub fn file_name(&self) -> String {
        unimplemented!("DirEntry::file_name");
    }
}

// TODO: impl Debug for DirEntry
