use super::get_file_type;
use super::oshandle::OsHandle;
use crate::handle::{Handle, HandleRights};
use crate::sys::osother::{OsOther, OsOtherExt};
use crate::wasi::{types, RightsExt};
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
        let handle = unsafe { OsHandle::from_raw_handle(file.into_raw_handle()) };
        Ok(Self::new(file_type, rights, handle))
    }
}

fn get_rights(file_type: &types::Filetype) -> io::Result<HandleRights> {
    let (base, inheriting) = match file_type {
        types::Filetype::BlockDevice => (
            types::Rights::block_device_base(),
            types::Rights::block_device_inheriting(),
        ),
        types::Filetype::CharacterDevice => (types::Rights::tty_base(), types::Rights::tty_base()),
        types::Filetype::SocketDgram | types::Filetype::SocketStream => (
            types::Rights::socket_base(),
            types::Rights::socket_inheriting(),
        ),
        types::Filetype::SymbolicLink | types::Filetype::Unknown => (
            types::Rights::regular_file_base(),
            types::Rights::regular_file_inheriting(),
        ),
        _ => unreachable!("these should have been handled already!"),
    };
    let rights = HandleRights::new(base, inheriting);
    Ok(rights)
}

impl OsOtherExt for OsOther {
    fn from_null() -> io::Result<Box<dyn Handle>> {
        let file = OpenOptions::new().read(true).write(true).open("NUL")?;
        let file = Self::try_from(file)?;
        Ok(Box::new(file))
    }
}
