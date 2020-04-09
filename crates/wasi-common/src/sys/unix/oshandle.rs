use crate::entry::EntryRights;
use crate::sys::oshandle::{AsFile, OsHandle, OsHandleExt};
use crate::wasi::{types, RightsExt};
use std::fs::{File, OpenOptions};
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FileTypeExt, FromRawFd, IntoRawFd, RawFd};

pub(crate) use super::sys_impl::osfile::*;

impl AsRawFd for OsHandle {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::OsFile(file) => file.as_raw_fd(),
            Self::Stdin => io::stdin().as_raw_fd(),
            Self::Stdout => io::stdout().as_raw_fd(),
            Self::Stderr => io::stderr().as_raw_fd(),
        }
    }
}

impl AsFile for OsHandle {
    fn as_file(&self) -> ManuallyDrop<File> {
        let file = unsafe { File::from_raw_fd(self.as_raw_fd()) };
        ManuallyDrop::new(file)
    }
}

impl From<File> for OsHandle {
    fn from(file: File) -> Self {
        Self::from(unsafe { OsFile::from_raw_fd(file.into_raw_fd()) })
    }
}

impl OsHandleExt for OsHandle {
    fn get_file_type(&self) -> io::Result<types::Filetype> {
        let file = self.as_file();
        let ft = file.metadata()?.file_type();
        let file_type = if ft.is_block_device() {
            log::debug!("Host fd {:?} is a block device", self.as_raw_fd());
            types::Filetype::BlockDevice
        } else if ft.is_char_device() {
            log::debug!("Host fd {:?} is a char device", self.as_raw_fd());
            types::Filetype::CharacterDevice
        } else if ft.is_dir() {
            log::debug!("Host fd {:?} is a directory", self.as_raw_fd());
            types::Filetype::Directory
        } else if ft.is_file() {
            log::debug!("Host fd {:?} is a file", self.as_raw_fd());
            types::Filetype::RegularFile
        } else if ft.is_socket() {
            log::debug!("Host fd {:?} is a socket", self.as_raw_fd());
            use yanix::socket::{get_socket_type, SockType};
            match unsafe { get_socket_type(self.as_raw_fd())? } {
                SockType::Datagram => types::Filetype::SocketDgram,
                SockType::Stream => types::Filetype::SocketStream,
                _ => return Err(io::Error::from_raw_os_error(libc::EINVAL)),
            }
        } else if ft.is_fifo() {
            log::debug!("Host fd {:?} is a fifo", self.as_raw_fd());
            types::Filetype::Unknown
        } else {
            log::debug!("Host fd {:?} is unknown", self.as_raw_fd());
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        };

        Ok(file_type)
    }

    fn get_rights(&self, file_type: types::Filetype) -> io::Result<EntryRights> {
        use yanix::{fcntl, file::OFlag};
        let (base, inheriting) = match file_type {
            types::Filetype::BlockDevice => (
                types::Rights::block_device_base(),
                types::Rights::block_device_inheriting(),
            ),
            types::Filetype::CharacterDevice => {
                use yanix::file::isatty;
                if unsafe { isatty(self.as_raw_fd())? } {
                    (types::Rights::tty_base(), types::Rights::tty_base())
                } else {
                    (
                        types::Rights::character_device_base(),
                        types::Rights::character_device_inheriting(),
                    )
                }
            }
            types::Filetype::Directory => (
                types::Rights::directory_base(),
                types::Rights::directory_inheriting(),
            ),
            types::Filetype::RegularFile => (
                types::Rights::regular_file_base(),
                types::Rights::regular_file_inheriting(),
            ),
            types::Filetype::SocketDgram | types::Filetype::SocketStream => (
                types::Rights::socket_base(),
                types::Rights::socket_inheriting(),
            ),
            types::Filetype::SymbolicLink | types::Filetype::Unknown => (
                types::Rights::regular_file_base(),
                types::Rights::regular_file_inheriting(),
            ),
        };
        let mut rights = EntryRights::new(base, inheriting);
        let flags = unsafe { fcntl::get_status_flags(self.as_raw_fd())? };
        let accmode = flags & OFlag::ACCMODE;
        if accmode == OFlag::RDONLY {
            rights.base &= !types::Rights::FD_WRITE;
        } else if accmode == OFlag::WRONLY {
            rights.base &= !types::Rights::FD_READ;
        }
        Ok(rights)
    }

    fn from_null() -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/null")?;
        Ok(Self::from(file))
    }
}
