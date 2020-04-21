use super::oshandle::OsHandle;
use crate::handle::{Handle, HandleRights};
use crate::sys::osfile::{OsFile, OsFileExt};
use crate::wasi::{types, RightsExt};
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle};

impl TryFrom<File> for OsFile {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let rights = get_rights(&file)?;
        let handle = unsafe { OsHandle::from_raw_handle(file.into_raw_handle()) };
        Ok(Self::new(rights, handle))
    }
}

fn get_rights(file: &File) -> io::Result<HandleRights> {
    use winx::file::{query_access_information, AccessMode};
    let mut rights = HandleRights::new(
        types::Rights::regular_file_base(),
        types::Rights::regular_file_inheriting(),
    );
    let mode = query_access_information(file.as_raw_handle())?;
    if mode.contains(AccessMode::FILE_GENERIC_READ) {
        rights.base |= types::Rights::FD_READ;
    }
    if mode.contains(AccessMode::FILE_GENERIC_WRITE) {
        rights.base |= types::Rights::FD_WRITE;
    }
    Ok(rights)
}

impl OsFileExt for OsFile {
    fn from_null() -> io::Result<Box<dyn Handle>> {
        let file = OpenOptions::new().read(true).write(true).open("NUL")?;
        let file = Self::try_from(file)?;
        Ok(Box::new(file))
    }
}
