use super::get_path::concatenate;
use crate::filesystem::primitives::FileType;
use std::ffi::OsString;
use std::path::Component;
use std::{fs, io};

pub(crate) fn read_dir(
    file: &fs::File,
) -> io::Result<impl Iterator<Item = io::Result<(OsString, FileType)>> + 'static> {
    let full_path = concatenate(file, Component::CurDir.as_ref())?;
    let iter = fs::read_dir(full_path)?;
    Ok(iter.map(|entry| {
        let entry = entry?;
        Ok((entry.file_name(), entry.file_type()?.into()))
    }))
}
