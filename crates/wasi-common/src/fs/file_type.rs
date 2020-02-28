/// A structure representing a type of file with accessors for each file type.
/// It is returned by `Metadata::file_type` method.
///
/// This corresponds to [`std::fs::FileType`].
///
/// TODO: Not yet implemented.
///
/// [`std::fs::FileType`]: https://doc.rust-lang.org/std/fs/struct.FileType.html
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileType {}

impl FileType {
    /// Tests whether this file type represents a directory.
    ///
    /// This corresponds to [`std::fs::FileType::is_dir`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::FileType::is_dir`]: https://doc.rust-lang.org/std/fs/struct.FileType.html#method.is_dir
    pub fn is_dir(&self) -> bool {
        unimplemented!("FileType::is_dir");
    }

    /// Tests whether this file type represents a regular file.
    ///
    /// This corresponds to [`std::fs::FileType::is_file`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::FileType::is_file`]: https://doc.rust-lang.org/std/fs/struct.FileType.html#method.is_file
    pub fn is_file(&self) -> bool {
        unimplemented!("FileType::is_file");
    }

    /// Tests whether this file type represents a symbolic link.
    ///
    /// This corresponds to [`std::fs::FileType::is_symlink`].
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::FileType::is_symlink`]: https://doc.rust-lang.org/std/fs/struct.FileType.html#method.is_symlink
    pub fn is_symlink(&self) -> bool {
        unimplemented!("FileType::is_symlink");
    }
}

// TODO: functions from FileTypeExt?

// TODO: impl Debug for FileType
