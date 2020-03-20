use crate::entry::{Descriptor, OsHandleRef};
use crate::wasi::{types, RightsExt};
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FileTypeExt, FromRawFd, RawFd};

pub(crate) use super::sys_impl::oshandle::*;

impl AsRawFd for Descriptor {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::OsHandle(file) => file.as_raw_fd(),
            Self::VirtualFile(_) => panic!("virtual files do not have a raw fd"),
            Self::Stdin => io::stdin().as_raw_fd(),
            Self::Stdout => io::stdout().as_raw_fd(),
            Self::Stderr => io::stderr().as_raw_fd(),
        }
    }
}

pub(crate) fn descriptor_as_oshandle<'lifetime>(
    desc: &'lifetime Descriptor,
) -> OsHandleRef<'lifetime> {
    OsHandleRef::new(ManuallyDrop::new(OsHandle::from(unsafe {
        File::from_raw_fd(desc.as_raw_fd())
    })))
}

/// Returns the set of all possible rights that are both relevant for the file
/// type and consistent with the open mode.
///
/// This function is unsafe because it operates on a raw file descriptor.
pub(crate) unsafe fn determine_type_and_access_rights<Fd: AsRawFd>(
    fd: &Fd,
) -> io::Result<(types::Filetype, types::Rights, types::Rights)> {
    let (file_type, mut rights_base, rights_inheriting) = determine_type_rights(fd)?;

    use yanix::{fcntl, file::OFlag};
    let flags = fcntl::get_status_flags(fd.as_raw_fd())?;
    let accmode = flags & OFlag::ACCMODE;
    if accmode == OFlag::RDONLY {
        rights_base &= !types::Rights::FD_WRITE;
    } else if accmode == OFlag::WRONLY {
        rights_base &= !types::Rights::FD_READ;
    }

    Ok((file_type, rights_base, rights_inheriting))
}

/// Returns the set of all possible rights that are relevant for file type.
///
/// This function is unsafe because it operates on a raw file descriptor.
pub(crate) unsafe fn determine_type_rights<Fd: AsRawFd>(
    fd: &Fd,
) -> io::Result<(types::Filetype, types::Rights, types::Rights)> {
    let (file_type, rights_base, rights_inheriting) = {
        // we just make a `File` here for convenience; we don't want it to close when it drops
        let file = std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd.as_raw_fd()));
        let ft = file.metadata()?.file_type();
        if ft.is_block_device() {
            log::debug!("Host fd {:?} is a block device", fd.as_raw_fd());
            (
                types::Filetype::BlockDevice,
                types::Rights::block_device_base(),
                types::Rights::block_device_inheriting(),
            )
        } else if ft.is_char_device() {
            log::debug!("Host fd {:?} is a char device", fd.as_raw_fd());
            use yanix::file::isatty;
            if isatty(fd.as_raw_fd())? {
                (
                    types::Filetype::CharacterDevice,
                    types::Rights::tty_base(),
                    types::Rights::tty_base(),
                )
            } else {
                (
                    types::Filetype::CharacterDevice,
                    types::Rights::character_device_base(),
                    types::Rights::character_device_inheriting(),
                )
            }
        } else if ft.is_dir() {
            log::debug!("Host fd {:?} is a directory", fd.as_raw_fd());
            (
                types::Filetype::Directory,
                types::Rights::directory_base(),
                types::Rights::directory_inheriting(),
            )
        } else if ft.is_file() {
            log::debug!("Host fd {:?} is a file", fd.as_raw_fd());
            (
                types::Filetype::RegularFile,
                types::Rights::regular_file_base(),
                types::Rights::regular_file_inheriting(),
            )
        } else if ft.is_socket() {
            log::debug!("Host fd {:?} is a socket", fd.as_raw_fd());
            use yanix::socket::{get_socket_type, SockType};
            match get_socket_type(fd.as_raw_fd())? {
                SockType::Datagram => (
                    types::Filetype::SocketDgram,
                    types::Rights::socket_base(),
                    types::Rights::socket_inheriting(),
                ),
                SockType::Stream => (
                    types::Filetype::SocketStream,
                    types::Rights::socket_base(),
                    types::Rights::socket_inheriting(),
                ),
                _ => return Err(io::Error::from_raw_os_error(libc::EINVAL)),
            }
        } else if ft.is_fifo() {
            log::debug!("Host fd {:?} is a fifo", fd.as_raw_fd());
            (
                types::Filetype::Unknown,
                types::Rights::regular_file_base(),
                types::Rights::regular_file_inheriting(),
            )
        } else {
            log::debug!("Host fd {:?} is unknown", fd.as_raw_fd());
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }
    };

    Ok((file_type, rights_base, rights_inheriting))
}
