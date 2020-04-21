use super::oshandle::{OsDirHandle, OsHandle};
use crate::handle::HandleRights;
use crate::sys::osdir::OsDir;
use crate::wasi::{types, RightsExt};
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle};

impl TryFrom<File> for OsDir {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let rights = get_rights(&file)?;
        let handle = unsafe { OsHandle::from_raw_handle(file.into_raw_handle()) };
        let handle = OsDirHandle::new(handle)?;
        Ok(Self::new(rights, handle))
    }
}

fn get_rights(file: &File) -> io::Result<HandleRights> {
    use winx::file::{query_access_information, AccessMode};
    let mut rights = HandleRights::new(
        types::Rights::directory_base(),
        types::Rights::directory_inheriting(),
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
