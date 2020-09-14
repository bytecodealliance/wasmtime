use super::oshandle::RawOsHandle;
use super::{get_file_type, get_rights};
use crate::handle::Filetype;
use crate::sys::osother::OsOther;
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::os::windows::prelude::{FromRawHandle, IntoRawHandle};

impl TryFrom<File> for OsOther {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let file_type = get_file_type(&file)?;
        if file_type == Filetype::RegularFile || file_type == Filetype::Directory {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }
        let rights = get_rights(&file_type)?;
        let handle = unsafe { RawOsHandle::from_raw_handle(file.into_raw_handle()) };
        Ok(Self::new(file_type, rights, handle))
    }
}
