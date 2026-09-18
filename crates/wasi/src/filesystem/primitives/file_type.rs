//! The `FileType` struct.

use std::fs;

/// A structure representing a type of file with accessors for each file type.
///
/// This corresponds to [`std::fs::FileType`].
///
/// <details>
/// We need to define our own version because the libstd `FileType` doesn't
/// have a public constructor that we can use.
/// </details>
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FileType {
    Std(fs::FileType),
    #[cfg(unix)]
    Unix(rustix::fs::FileType),
}

impl From<fs::FileType> for FileType {
    fn from(std: fs::FileType) -> Self {
        Self::Std(std)
    }
}

impl FileType {
    /// Tests whether this file type represents a directory.
    ///
    /// This corresponds to [`std::fs::FileType::is_dir`].
    #[inline]
    pub fn is_dir(&self) -> bool {
        match self {
            Self::Std(std) => std.is_dir(),
            #[cfg(unix)]
            Self::Unix(unix) => unix.is_dir(),
        }
    }

    /// Tests whether this file type represents a regular file.
    ///
    /// This corresponds to [`std::fs::FileType::is_file`].
    #[inline]
    pub fn is_file(&self) -> bool {
        match self {
            Self::Std(std) => std.is_file(),
            #[cfg(unix)]
            Self::Unix(unix) => unix.is_file(),
        }
    }

    /// Tests whether this file type represents a symbolic link.
    ///
    /// This corresponds to [`std::fs::FileType::is_symlink`].
    #[inline]
    pub fn is_symlink(&self) -> bool {
        match self {
            Self::Std(std) => std.is_symlink(),
            #[cfg(unix)]
            Self::Unix(unix) => unix.is_symlink(),
        }
    }
}
