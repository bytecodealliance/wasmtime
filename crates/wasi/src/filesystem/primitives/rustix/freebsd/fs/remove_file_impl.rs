use crate::filesystem::primitives::via_parent;
use rustix::fs::{AtFlags, unlinkat};
use std::path::Path;
use std::{fs, io};

pub(crate) fn remove_file_impl(start: &fs::File, path: &Path) -> io::Result<()> {
    if !super::beneath_supported() {
        return via_parent::remove_file(start, path);
    }

    Ok(unlinkat(start, path, AtFlags::RESOLVE_BENEATH)?)
}
