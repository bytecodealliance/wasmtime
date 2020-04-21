use super::oshandle::{OsDirHandle, OsHandle};
use crate::handle::HandleRights;
use crate::sys::osdir::OsDir;
use crate::wasi::{types, RightsExt};
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd};

impl TryFrom<File> for OsDir {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let rights = get_rights(&file)?;
        let handle = unsafe { OsHandle::from_raw_fd(file.into_raw_fd()) };
        let handle = OsDirHandle::new(handle)?;
        Ok(Self::new(rights, handle))
    }
}

fn get_rights(file: &File) -> io::Result<HandleRights> {
    use yanix::{fcntl, file::OFlag};
    let mut rights = HandleRights::new(
        types::Rights::directory_base(),
        types::Rights::directory_inheriting(),
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
