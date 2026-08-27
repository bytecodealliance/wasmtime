//! The `FileType` struct.

use crate::filesystem::primitives::ImplFileTypeExt;

/// `FileType`'s inner state.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Inner {
    /// A directory.
    Dir,

    /// A file.
    File,

    /// An unknown entity.
    Unknown,

    /// A `FileTypeExt` type.
    Ext(ImplFileTypeExt),
}

/// A structure representing a type of file with accessors for each file type.
///
/// This corresponds to [`std::fs::FileType`].
///
/// <details>
/// We need to define our own version because the libstd `FileType` doesn't
/// have a public constructor that we can use.
/// </details>
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct FileType(Inner);

impl FileType {
    /// Creates a `FileType` for which `is_dir()` returns `true`.
    #[inline]
    pub const fn dir() -> Self {
        Self(Inner::Dir)
    }

    /// Creates a `FileType` for which `is_file()` returns `true`.
    #[inline]
    pub const fn file() -> Self {
        Self(Inner::File)
    }

    /// Creates a `FileType` for which `is_unknown()` returns `true`.
    #[inline]
    pub const fn unknown() -> Self {
        Self(Inner::Unknown)
    }

    /// Creates a `FileType` from extension type.
    #[inline]
    pub(crate) const fn ext(ext: ImplFileTypeExt) -> Self {
        Self(Inner::Ext(ext))
    }

    /// Tests whether this file type represents a directory.
    ///
    /// This corresponds to [`std::fs::FileType::is_dir`].
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.0 == Inner::Dir
    }

    /// Tests whether this file type represents a regular file.
    ///
    /// This corresponds to [`std::fs::FileType::is_file`].
    #[inline]
    pub fn is_file(&self) -> bool {
        self.0 == Inner::File
    }

    /// Tests whether this file type represents a symbolic link.
    ///
    /// This corresponds to [`std::fs::FileType::is_symlink`].
    #[inline]
    pub fn is_symlink(&self) -> bool {
        if let Inner::Ext(ext) = self.0 {
            ext.is_symlink()
        } else {
            false
        }
    }
}

/// Unix-specific extensions for [`FileType`].
///
/// This corresponds to [`std::os::unix::fs::FileTypeExt`].
#[cfg(any(unix, target_os = "vxworks"))]
pub trait FileTypeExt {
    /// Returns `true` if this file type is a block device.
    fn is_block_device(&self) -> bool;
    /// Returns `true` if this file type is a character device.
    fn is_char_device(&self) -> bool;
}

#[cfg(any(unix, target_os = "vxworks"))]
impl FileTypeExt for FileType {
    #[inline]
    fn is_block_device(&self) -> bool {
        self.0 == Inner::Ext(ImplFileTypeExt::block_device())
    }

    #[inline]
    fn is_char_device(&self) -> bool {
        self.0 == Inner::Ext(ImplFileTypeExt::char_device())
    }
}

/// Extension trait to allow `is_block_device` etc. to be exposed by
/// the `cap-fs-ext` crate.
///
/// This is hidden from the main API since this functionality isn't present in
/// `std`. Use `cap_fs_ext::FileTypeExt` instead of calling this directly.
#[cfg(windows)]
#[doc(hidden)]
pub trait _WindowsFileTypeExt {
    fn is_block_device(&self) -> bool;
    fn is_char_device(&self) -> bool;
    fn is_fifo(&self) -> bool;
    fn is_socket(&self) -> bool;
}
