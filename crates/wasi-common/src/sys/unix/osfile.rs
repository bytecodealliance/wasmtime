use super::oshandle::OsHandle;
use crate::handle::{Handle, HandleRights};
use crate::sys::osfile::{OsFile, OsFileExt};
use crate::wasi::{types, RightsExt};
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd};

impl TryFrom<File> for OsFile {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let rights = get_rights(&file)?;
        let handle = unsafe { OsHandle::from_raw_fd(file.into_raw_fd()) };
        Ok(Self::new(rights, handle))
    }
}

fn get_rights(file: &File) -> io::Result<HandleRights> {
    use yanix::{fcntl, file::OFlag};
    let mut rights = HandleRights::new(
        types::Rights::regular_file_base(),
        types::Rights::regular_file_inheriting(),
    );
    let flags = unsafe { fcntl::get_status_flags(file.as_raw_fd())? };
    let accmode = flags & OFlag::ACCMODE;
    if accmode == OFlag::RDONLY {
        rights.base &= !types::Rights::FD_WRITE;
    } else if accmode == OFlag::WRONLY {
        rights.base &= !types::Rights::FD_READ;
    }
    Ok(rights)
}

impl OsFileExt for OsFile {
    fn from_null() -> io::Result<Box<dyn Handle>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/null")?;
        let file = Self::try_from(file)?;
        Ok(Box::new(file))
    }
}
