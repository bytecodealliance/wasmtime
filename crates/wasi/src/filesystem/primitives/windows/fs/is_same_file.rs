use crate::filesystem::primitives::ImplMetadataExt;
#[cfg(windows_by_handle)]
use crate::filesystem::primitives::Metadata;
use std::{fs, io};

/// Determine if `a` and `b` refer to the same inode on the same device.
pub(crate) fn is_same_file(a: &fs::File, b: &fs::File) -> io::Result<bool> {
    let a_metadata = ImplMetadataExt::from(a, &a.metadata()?)?;
    let b_metadata = ImplMetadataExt::from(b, &b.metadata()?)?;
    Ok(a_metadata.is_same_file(&b_metadata))
}

/// Determine if `a` and `b` are metadata for the same inode on the same
/// device.
#[cfg(windows_by_handle)]
#[allow(dead_code)]
pub(crate) fn is_same_file_metadata(a: &Metadata, b: &Metadata) -> io::Result<bool> {
    use crate::filesystem::primitives::MetadataExt;
    Ok(a.volume_serial_number() == b.volume_serial_number() && a.file_index() == b.file_index())
}
