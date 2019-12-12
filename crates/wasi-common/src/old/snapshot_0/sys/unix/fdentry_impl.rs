use crate::old::snapshot_0::fdentry::{Descriptor, OsHandleRef};
use crate::old::snapshot_0::{sys::unix::sys_impl, wasi, Error, Result};
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FileTypeExt, FromRawFd, RawFd};

pub(crate) use sys_impl::oshandle::*;

impl AsRawFd for Descriptor {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::OsHandle(file) => file.as_raw_fd(),
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

/// This function is unsafe because it operates on a raw file descriptor.
pub(crate) unsafe fn determine_type_and_access_rights<Fd: AsRawFd>(
    fd: &Fd,
) -> Result<(
    wasi::__wasi_filetype_t,
    wasi::__wasi_rights_t,
    wasi::__wasi_rights_t,
)> {
    let (file_type, mut rights_base, rights_inheriting) = determine_type_rights(fd)?;

    use yanix::{fcntl, file::OFlag};
    let flags = fcntl::get_status_flags(fd.as_raw_fd())?;
    let accmode = flags & OFlag::ACCMODE;
    if accmode == OFlag::RDONLY {
        rights_base &= !wasi::__WASI_RIGHTS_FD_WRITE;
    } else if accmode == OFlag::WRONLY {
        rights_base &= !wasi::__WASI_RIGHTS_FD_READ;
    }

    Ok((file_type, rights_base, rights_inheriting))
}

/// This function is unsafe because it operates on a raw file descriptor.
pub(crate) unsafe fn determine_type_rights<Fd: AsRawFd>(
    fd: &Fd,
) -> Result<(
    wasi::__wasi_filetype_t,
    wasi::__wasi_rights_t,
    wasi::__wasi_rights_t,
)> {
    let (file_type, rights_base, rights_inheriting) = {
        // we just make a `File` here for convenience; we don't want it to close when it drops
        let file = std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd.as_raw_fd()));
        let ft = file.metadata()?.file_type();
        if ft.is_block_device() {
            log::debug!("Host fd {:?} is a block device", fd.as_raw_fd());
            (
                wasi::__WASI_FILETYPE_BLOCK_DEVICE,
                wasi::RIGHTS_BLOCK_DEVICE_BASE,
                wasi::RIGHTS_BLOCK_DEVICE_INHERITING,
            )
        } else if ft.is_char_device() {
            log::debug!("Host fd {:?} is a char device", fd.as_raw_fd());
            use yanix::file::isatty;
            if isatty(fd.as_raw_fd())? {
                (
                    wasi::__WASI_FILETYPE_CHARACTER_DEVICE,
                    wasi::RIGHTS_TTY_BASE,
                    wasi::RIGHTS_TTY_BASE,
                )
            } else {
                (
                    wasi::__WASI_FILETYPE_CHARACTER_DEVICE,
                    wasi::RIGHTS_CHARACTER_DEVICE_BASE,
                    wasi::RIGHTS_CHARACTER_DEVICE_INHERITING,
                )
            }
        } else if ft.is_dir() {
            log::debug!("Host fd {:?} is a directory", fd.as_raw_fd());
            (
                wasi::__WASI_FILETYPE_DIRECTORY,
                wasi::RIGHTS_DIRECTORY_BASE,
                wasi::RIGHTS_DIRECTORY_INHERITING,
            )
        } else if ft.is_file() {
            log::debug!("Host fd {:?} is a file", fd.as_raw_fd());
            (
                wasi::__WASI_FILETYPE_REGULAR_FILE,
                wasi::RIGHTS_REGULAR_FILE_BASE,
                wasi::RIGHTS_REGULAR_FILE_INHERITING,
            )
        } else if ft.is_socket() {
            log::debug!("Host fd {:?} is a socket", fd.as_raw_fd());
            use yanix::socket::{get_socket_type, SockType};
            match get_socket_type(fd.as_raw_fd())? {
                SockType::Datagram => (
                    wasi::__WASI_FILETYPE_SOCKET_DGRAM,
                    wasi::RIGHTS_SOCKET_BASE,
                    wasi::RIGHTS_SOCKET_INHERITING,
                ),
                SockType::Stream => (
                    wasi::__WASI_FILETYPE_SOCKET_STREAM,
                    wasi::RIGHTS_SOCKET_BASE,
                    wasi::RIGHTS_SOCKET_INHERITING,
                ),
                _ => return Err(Error::EINVAL),
            }
        } else if ft.is_fifo() {
            log::debug!("Host fd {:?} is a fifo", fd.as_raw_fd());
            (
                wasi::__WASI_FILETYPE_UNKNOWN,
                wasi::RIGHTS_REGULAR_FILE_BASE,
                wasi::RIGHTS_REGULAR_FILE_INHERITING,
            )
        } else {
            log::debug!("Host fd {:?} is unknown", fd.as_raw_fd());
            return Err(Error::EINVAL);
        }
    };

    Ok((file_type, rights_base, rights_inheriting))
}
