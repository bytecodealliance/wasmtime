#![allow(clippy::useless_conversion)]

use crate::filesystem::primitives::MetadataExt;
use std::{fs, io};

#[derive(Debug, Clone)]
pub(crate) struct ImplMetadataExt {
    file_attributes: u32,
    number_of_links: Option<u32>,
}

impl ImplMetadataExt {
    /// Constructs a new instance of `Self` from the given [`std::fs::File`]
    /// and [`std::fs::Metadata`].
    #[inline]
    pub(crate) fn from(file: &fs::File, std: &fs::Metadata) -> io::Result<Self> {
        let fileinfo = winx::winapi_util::file::information(file)?;
        let t64: u64 = fileinfo.number_of_links();
        let t32: u32 = t64.try_into().unwrap();

        Ok(Self::from_parts(std, Some(t32)))
    }

    /// Constructs a new instance of `Self` from the given
    /// [`std::fs::Metadata`].
    ///
    /// As with the comments in [`std::fs::Metadata::volume_serial_number`] and
    /// nearby functions, some fields of the resulting metadata will be `None`.
    ///
    /// [`std::fs::Metadata::volume_serial_number`]: https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html#tymethod.volume_serial_number
    #[inline]
    pub(crate) fn from_just_metadata(std: &fs::Metadata) -> Self {
        Self::from_parts(std, None)
    }

    #[inline]
    fn from_parts(std: &fs::Metadata, number_of_links: Option<u32>) -> Self {
        use std::os::windows::fs::MetadataExt;
        Self {
            file_attributes: std.file_attributes(),
            number_of_links,
        }
    }

    /// `MetadataExt` requires nightly to be implemented, but we sometimes
    /// just need the file attributes.
    #[inline]
    pub(crate) fn file_attributes(&self) -> u32 {
        self.file_attributes
    }
}

impl MetadataExt for ImplMetadataExt {
    fn file_attributes(&self) -> u32 {
        self.file_attributes
    }
}

#[doc(hidden)]
impl crate::filesystem::primitives::_WindowsByHandle for crate::filesystem::primitives::Metadata {
    #[inline]
    fn number_of_links(&self) -> Option<u32> {
        self.ext.number_of_links
    }
}
