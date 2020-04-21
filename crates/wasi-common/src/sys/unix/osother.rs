use super::get_file_type;
use super::oshandle::OsHandle;
use crate::handle::HandleRights;
use crate::sys::osother::OsOther;
use crate::wasi::{types, RightsExt};
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd};

impl TryFrom<File> for OsOther {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let file_type = get_file_type(&file)?;
        let rights = get_rights(&file, &file_type)?;
        let handle = unsafe { OsHandle::from_raw_fd(file.into_raw_fd()) };
        Ok(Self::new(file_type, rights, handle))
    }
}

fn get_rights(file: &File, file_type: &types::Filetype) -> io::Result<HandleRights> {
    use yanix::{fcntl, file::OFlag};
    let (base, inheriting) = match file_type {
        types::Filetype::BlockDevice => (
            types::Rights::block_device_base(),
            types::Rights::block_device_inheriting(),
        ),
        types::Filetype::CharacterDevice => {
            use yanix::file::isatty;
            if unsafe { isatty(file.as_raw_fd())? } {
                (types::Rights::tty_base(), types::Rights::tty_base())
            } else {
                (
                    types::Rights::character_device_base(),
                    types::Rights::character_device_inheriting(),
                )
            }
        }
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
    let mut rights = HandleRights::new(base, inheriting);
    let flags = unsafe { fcntl::get_status_flags(file.as_raw_fd())? };
    let accmode = flags & OFlag::ACCMODE;
    if accmode == OFlag::RDONLY {
        rights.base &= !types::Rights::FD_WRITE;
    } else if accmode == OFlag::WRONLY {
        rights.base &= !types::Rights::FD_READ;
    }
    Ok(rights)
}
