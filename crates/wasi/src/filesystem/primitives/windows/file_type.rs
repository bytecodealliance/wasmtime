//! The `FileType` struct.

use std::fs;
use std::os::windows::fs::FileTypeExt;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
};

/// Bit set in reparse tags whose reparse point stands in for another named
/// entity (symlinks and junctions), as tested by `IsReparseTagNameSurrogate`
/// in `winnt.h`. This mirrors libstd's interpretation of reparse tags.
const REPARSE_TAG_NAME_SURROGATE: u32 = 0x2000_0000;

/// A structure representing a type of file with accessors for each file type.
///
/// This corresponds to [`std::fs::FileType`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FileType {
    is_directory: bool,
    is_symlink: bool,
}

impl From<fs::FileType> for FileType {
    fn from(std: fs::FileType) -> Self {
        Self {
            is_directory: std.is_dir() || std.is_symlink_dir(),
            is_symlink: std.is_symlink(),
        }
    }
}

impl FileType {
    /// Creates a `FileType` from a file's `FILE_ATTRIBUTE_*` flags and its
    /// reparse tag, in the same manner as libstd.
    pub(super) fn new(attributes: u32, reparse_tag: u32) -> Self {
        let is_reparse_point = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        let is_reparse_tag_name_surrogate = reparse_tag & REPARSE_TAG_NAME_SURROGATE != 0;
        Self {
            is_directory: attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
            is_symlink: is_reparse_point && is_reparse_tag_name_surrogate,
        }
    }

    /// Tests whether this file type represents a directory.
    ///
    /// This corresponds to [`std::fs::FileType::is_dir`].
    #[inline]
    pub fn is_dir(&self) -> bool {
        !self.is_symlink && self.is_directory
    }

    /// Tests whether this file type represents a regular file.
    ///
    /// This corresponds to [`std::fs::FileType::is_file`].
    #[inline]
    pub fn is_file(&self) -> bool {
        !self.is_symlink && !self.is_directory
    }

    /// Tests whether this file type represents a symbolic link.
    ///
    /// This corresponds to [`std::fs::FileType::is_symlink`].
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.is_symlink
    }

    /// Tests whether this file type represents a character device.
    ///
    /// Like libstd, this file type only distinguishes files, directories, and
    /// symlinks, so this always returns `false`.
    #[inline]
    pub fn is_char_device(&self) -> bool {
        false
    }

    /// Tests whether this file type represents a block device.
    ///
    /// Windows doesn't expose block devices as files, so this always returns
    /// `false`.
    #[inline]
    pub fn is_block_device(&self) -> bool {
        false
    }
}
