use crate::filesystem::primitives::ImplMetadataExt;
use std::{fs, io};

/// Determine if `a` and `b` refer to the same inode on the same device.
pub(crate) fn is_same_file(a: &fs::File, b: &fs::File) -> io::Result<bool> {
    let a_metadata = ImplMetadataExt::from(a, &a.metadata()?)?;
    let b_metadata = ImplMetadataExt::from(b, &b.metadata()?)?;
    Ok(a_metadata.is_same_file(&b_metadata))
}
