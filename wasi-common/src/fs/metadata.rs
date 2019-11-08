use crate::fs::{FileType, Permissions};
use std::{io, time::SystemTime};

/// Metadata information about a file.
///
/// This corresponds to [`std::fs::Metadata`].
///
/// TODO: Not yet implemented.
///
/// [`std::fs::Metadata`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html
#[derive(Clone)]
pub struct Metadata {}

impl Metadata {
    /// Returns the file type for this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::file_type`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::file_type`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.file_type
    pub fn file_type(&self) -> FileType {
        unimplemented!("Metadata::file_type");
    }

    /// Returns true if this metadata is for a directory.
    ///
    /// This corresponds to [`std::fs::Metadata::is_dir`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::is_dir`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.is_dir
    pub fn is_dir(&self) -> bool {
        unimplemented!("Metadata::is_dir");
    }

    /// Returns true if this metadata is for a regular file.
    ///
    /// This corresponds to [`std::fs::Metadata::is_file`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::is_file`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.is_file
    pub fn is_file(&self) -> bool {
        unimplemented!("Metadata::is_file");
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    ///
    /// This corresponds to [`std::fs::Metadata::len`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::len`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.len
    pub fn len(&self) -> u64 {
        unimplemented!("Metadata::len");
    }

    /// Returns the permissions of the file this metadata is for.
    ///
    /// This corresponds to [`std::fs::Metadata::permissions`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::permissions`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.permissions
    pub fn permissions(&self) -> Permissions {
        unimplemented!("Metadata::permissions");
    }

    /// Returns the last modification time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::modified`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::modified`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.modified
    pub fn modified(&self) -> io::Result<SystemTime> {
        unimplemented!("Metadata::modified");
    }

    /// Returns the last access time of this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::accessed`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::accessed`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.accessed
    pub fn accessed(&self) -> io::Result<SystemTime> {
        unimplemented!("Metadata::accessed");
    }

    /// Returns the creation time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::created`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::Metadata::created`]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.created
    pub fn created(&self) -> io::Result<SystemTime> {
        unimplemented!("Metadata::created");
    }
}

// TODO: Functions from MetadataExt?

// TODO: impl Debug for Metadata
