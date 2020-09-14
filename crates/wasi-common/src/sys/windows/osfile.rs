use super::oshandle::RawOsHandle;
use crate::handle::{HandleRights, Rights, RightsExt};
use crate::sys::osfile::OsFile;
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle};

impl TryFrom<File> for OsFile {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let ft = file.metadata()?.file_type();
        if !ft.is_file() {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }
        let rights = get_rights(&file)?;
        let handle = unsafe { RawOsHandle::from_raw_handle(file.into_raw_handle()) };
        Ok(Self::new(rights, handle))
    }
}

fn get_rights(file: &File) -> io::Result<HandleRights> {
    use winx::file::{query_access_information, AccessMode};
    let mut rights = HandleRights::new(
        Rights::regular_file_base(),
        Rights::regular_file_inheriting(),
    );
    let mode = query_access_information(file.as_raw_handle())?;
    if mode.contains(AccessMode::FILE_GENERIC_READ) {
        rights.base |= Rights::FD_READ;
    }
    if mode.contains(AccessMode::FILE_GENERIC_WRITE) {
        rights.base |= Rights::FD_WRITE;
    }
    Ok(rights)
}
