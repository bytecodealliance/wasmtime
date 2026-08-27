use crate::filesystem::primitives::{FileType, ImplFileTypeExt, ImplMetadataExt};
use std::time::SystemTime;
use std::{fs, io};

/// Metadata information about a file.
///
/// This corresponds to [`std::fs::Metadata`].
///
/// <details>
/// We need to define our own version because the libstd `Metadata` doesn't
/// have a public constructor that we can use.
/// </details>
#[derive(Debug, Clone)]
pub struct Metadata {
    pub(crate) file_type: FileType,
    pub(crate) len: u64,
    pub(crate) modified: Option<SystemTime>,
    pub(crate) accessed: Option<SystemTime>,
    pub(crate) created: Option<SystemTime>,
    pub(crate) ext: ImplMetadataExt,
}

#[allow(clippy::len_without_is_empty)]
impl Metadata {
    /// Constructs a new instance of `Self` from the given [`std::fs::File`].
    #[inline]
    pub fn from_file(file: &fs::File) -> io::Result<Self> {
        let std = file.metadata()?;
        let ext = ImplMetadataExt::from(file, &std)?;
        let file_type = ImplFileTypeExt::from(file, &std)?;
        Ok(Self::from_parts(std, ext, file_type))
    }

    /// Constructs a new instance of `Self` from the given
    /// [`std::fs::Metadata`].
    ///
    /// As with the comments in [`std::fs::Metadata::volume_serial_number`] and
    /// nearby functions, some fields of the resulting metadata will be `None`.
    ///
    /// [`std::fs::Metadata::volume_serial_number`]: https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html#tymethod.volume_serial_number
    #[inline]
    pub fn from_just_metadata(std: fs::Metadata) -> Self {
        let ext = ImplMetadataExt::from_just_metadata(&std);
        let file_type = ImplFileTypeExt::from_just_metadata(&std);
        Self::from_parts(std, ext, file_type)
    }

    #[inline]
    fn from_parts(std: fs::Metadata, ext: ImplMetadataExt, file_type: FileType) -> Self {
        Self {
            file_type,
            len: std.len(),
            modified: std.modified().ok(),
            accessed: std.accessed().ok(),
            created: std.created().ok(),
            ext,
        }
    }

    /// Returns the file type for this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::file_type`].
    #[inline]
    pub const fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Returns `true` if this metadata is for a directory.
    ///
    /// This corresponds to [`std::fs::Metadata::is_dir`].
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    ///
    /// This corresponds to [`std::fs::Metadata::len`].
    #[inline]
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Returns the last modification time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::modified`].
    #[inline]
    pub fn modified(&self) -> io::Result<SystemTime> {
        self.modified.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "modified time metadata not available on this platform",
            )
        })
    }

    /// Returns the last access time of this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::accessed`].
    #[inline]
    pub fn accessed(&self) -> io::Result<SystemTime> {
        self.accessed.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "accessed time metadata not available on this platform",
            )
        })
    }

    /// Returns the creation time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::created`].
    #[inline]
    pub fn created(&self) -> io::Result<SystemTime> {
        self.created.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "created time metadata not available on this platform",
            )
        })
    }

    /// `MetadataExt` requires nightly to be implemented, but we sometimes
    /// just need the file attributes.
    #[cfg(windows)]
    #[inline]
    pub(crate) fn file_attributes(&self) -> u32 {
        self.ext.file_attributes()
    }
}

/// Unix-specific extensions for [`MetadataExt`].
///
/// This corresponds to [`std::os::unix::fs::MetadataExt`].
#[cfg(any(unix, target_os = "vxworks"))]
pub trait MetadataExt {
    /// Returns the ID of the device containing the file.
    fn dev(&self) -> u64;
    /// Returns the inode number.
    fn ino(&self) -> u64;
    /// Returns the number of hard links pointing to this file.
    fn nlink(&self) -> u64;
    #[cfg(target_os = "vxworks")]
    fn attrib(&self) -> u8;
}

/// WASI-specific extensions for [`MetadataExt`].
///
/// This corresponds to [`std::os::wasi::fs::MetadataExt`].
#[cfg(target_os = "wasi")]
pub trait MetadataExt {
    /// Returns the ID of the device containing the file.
    fn dev(&self) -> u64;
    /// Returns the inode number.
    fn ino(&self) -> u64;
    /// Returns the number of hard links pointing to this file.
    fn nlink(&self) -> u64;
}

/// Windows-specific extensions to [`Metadata`].
///
/// This corresponds to [`std::os::windows::fs::MetadataExt`].
#[cfg(windows)]
pub trait MetadataExt {
    /// Returns the value of the `dwFileAttributes` field of this metadata.
    fn file_attributes(&self) -> u32;
}

#[cfg(unix)]
impl MetadataExt for Metadata {
    #[inline]
    fn dev(&self) -> u64 {
        crate::filesystem::primitives::MetadataExt::dev(&self.ext)
    }

    #[inline]
    fn ino(&self) -> u64 {
        crate::filesystem::primitives::MetadataExt::ino(&self.ext)
    }

    #[inline]
    fn nlink(&self) -> u64 {
        crate::filesystem::primitives::MetadataExt::nlink(&self.ext)
    }
}

#[cfg(target_os = "wasi")]
impl MetadataExt for Metadata {
    #[inline]
    fn dev(&self) -> u64 {
        crate::filesystem::primitives::MetadataExt::dev(&self.ext)
    }

    #[inline]
    fn ino(&self) -> u64 {
        crate::filesystem::primitives::MetadataExt::ino(&self.ext)
    }

    #[inline]
    fn nlink(&self) -> u64 {
        crate::filesystem::primitives::MetadataExt::nlink(&self.ext)
    }
}

#[cfg(target_os = "vxworks")]
impl MetadataExt for Metadata {
    #[inline]
    fn dev(&self) -> u64 {
        self.ext.dev()
    }

    #[inline]
    fn ino(&self) -> u64 {
        self.ext.ino()
    }

    #[inline]
    fn nlink(&self) -> u64 {
        self.ext.nlink()
    }
}

#[cfg(windows)]
impl MetadataExt for Metadata {
    #[inline]
    fn file_attributes(&self) -> u32 {
        self.ext.file_attributes()
    }
}

/// Extension trait to allow `volume_serial_number` etc. to be exposed by
/// the `cap-fs-ext` crate.
///
/// This is hidden from the main API since this functionality isn't present in
/// `std`. Use `cap_fs_ext::MetadataExt` instead of calling this directly.
#[cfg(windows)]
#[doc(hidden)]
pub trait _WindowsByHandle {
    fn number_of_links(&self) -> Option<u32>;
}
