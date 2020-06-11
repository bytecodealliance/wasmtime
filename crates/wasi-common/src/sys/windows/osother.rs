use super::oshandle::RawOsHandle;
use super::{get_file_type, get_rights};
use crate::handle::Handle;
use crate::sys::osother::{OsOther, OsOtherExt};
use crate::wasi::types;
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::prelude::{FromRawHandle, IntoRawHandle};

impl TryFrom<File> for OsOther {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let file_type = get_file_type(&file)?;
        if file_type == types::Filetype::RegularFile || file_type == types::Filetype::Directory {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }
        let rights = get_rights(&file_type)?;
        let handle = unsafe { RawOsHandle::from_raw_handle(file.into_raw_handle()) };
        Ok(Self::new(file_type, rights, handle))
    }
}

impl OsOtherExt for OsOther {
    fn from_null() -> io::Result<Box<dyn Handle>> {
        let file = OpenOptions::new().read(true).write(true).open("NUL")?;
        let file = Self::try_from(file)?;
        Ok(Box::new(file))
    }
}
