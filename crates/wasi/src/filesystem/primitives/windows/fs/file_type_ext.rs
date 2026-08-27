use crate::filesystem::primitives::FileType;
use std::{fs, io};

/// A type that implements `FileTypeExt` for this platform.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) enum ImplFileTypeExt {
    CharacterDevice,
    Fifo,
    SymlinkUnknown,
}

impl ImplFileTypeExt {
    /// Constructs a new instance of `Self` from the given [`std::fs::File`]
    /// and [`std::fs::Metadata`].
    pub(crate) fn from(file: &fs::File, metadata: &fs::Metadata) -> io::Result<FileType> {
        // Check for the things we can do with just metadata.
        let file_type = Self::from_just_metadata(metadata);
        if file_type != FileType::unknown() {
            return Ok(file_type);
        }

        // Use the open file to check for one of the exotic file types.
        let file_type = winx::winapi_util::file::typ(file)?;
        if file_type.is_char() {
            return Ok(FileType::ext(ImplFileTypeExt::CharacterDevice));
        }
        if file_type.is_pipe() {
            return Ok(FileType::ext(ImplFileTypeExt::Fifo));
        }

        Ok(FileType::unknown())
    }

    /// Constructs a new instance of `Self` from the given
    /// [`std::fs::Metadata`].
    #[inline]
    pub(crate) fn from_just_metadata(metadata: &fs::Metadata) -> FileType {
        let std = metadata.file_type();
        Self::from_std(std)
    }

    /// Constructs a new instance of `Self` from the given
    /// [`std::fs::FileType`].
    #[inline]
    pub(crate) fn from_std(std: fs::FileType) -> FileType {
        if std.is_file() {
            return FileType::file();
        }
        if std.is_dir() {
            return FileType::dir();
        }

        if std.is_symlink() {
            return FileType::ext(Self::SymlinkUnknown);
        }

        FileType::unknown()
    }

    #[inline]
    pub(crate) fn is_symlink(&self) -> bool {
        match self {
            Self::SymlinkUnknown => true,
            _ => false,
        }
    }
}

#[doc(hidden)]
impl crate::filesystem::primitives::_WindowsFileTypeExt
    for crate::filesystem::primitives::FileType
{
    #[inline]
    fn is_block_device(&self) -> bool {
        false
    }

    #[inline]
    fn is_char_device(&self) -> bool {
        *self == FileType::ext(ImplFileTypeExt::CharacterDevice)
    }

    #[inline]
    fn is_fifo(&self) -> bool {
        *self == FileType::ext(ImplFileTypeExt::Fifo)
    }

    #[inline]
    fn is_socket(&self) -> bool {
        false
    }
}
