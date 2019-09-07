use crate::fdentry::Descriptor;
use crate::{host, Error, Result};
use std::io;
use std::os::unix::prelude::{AsRawFd, FileTypeExt, FromRawFd, RawFd};

impl AsRawFd for Descriptor {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Descriptor::File(f) => f.as_raw_fd(),
            Descriptor::Stdin => io::stdin().as_raw_fd(),
            Descriptor::Stdout => io::stdout().as_raw_fd(),
            Descriptor::Stderr => io::stderr().as_raw_fd(),
        }
    }
}

pub(crate) fn determine_type_and_access_rights<Fd: AsRawFd>(
    fd: &Fd,
) -> Result<(
    host::__wasi_filetype_t,
    host::__wasi_rights_t,
    host::__wasi_rights_t,
)> {
    let (file_type, mut rights_base, rights_inheriting) = determine_type_rights(fd)?;

    use nix::fcntl::{fcntl, OFlag, F_GETFL};
    let flags_bits = fcntl(fd.as_raw_fd(), F_GETFL)?;
    let flags = OFlag::from_bits_truncate(flags_bits);
    let accmode = flags & OFlag::O_ACCMODE;
    if accmode == OFlag::O_RDONLY {
        rights_base &= !host::__WASI_RIGHT_FD_WRITE;
    } else if accmode == OFlag::O_WRONLY {
        rights_base &= !host::__WASI_RIGHT_FD_READ;
    }

    Ok((file_type, rights_base, rights_inheriting))
}

pub(crate) fn determine_type_rights<Fd: AsRawFd>(
    fd: &Fd,
) -> Result<(
    host::__wasi_filetype_t,
    host::__wasi_rights_t,
    host::__wasi_rights_t,
)> {
    let (file_type, rights_base, rights_inheriting) = {
        // we just make a `File` here for convenience; we don't want it to close when it drops
        let file =
            std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd.as_raw_fd()) });
        let ft = file.metadata()?.file_type();
        if ft.is_block_device() {
            (
                host::__WASI_FILETYPE_BLOCK_DEVICE,
                host::RIGHTS_BLOCK_DEVICE_BASE,
                host::RIGHTS_BLOCK_DEVICE_INHERITING,
            )
        } else if ft.is_char_device() {
            if nix::unistd::isatty(fd.as_raw_fd())? {
                (
                    host::__WASI_FILETYPE_CHARACTER_DEVICE,
                    host::RIGHTS_TTY_BASE,
                    host::RIGHTS_TTY_BASE,
                )
            } else {
                (
                    host::__WASI_FILETYPE_CHARACTER_DEVICE,
                    host::RIGHTS_CHARACTER_DEVICE_BASE,
                    host::RIGHTS_CHARACTER_DEVICE_INHERITING,
                )
            }
        } else if ft.is_dir() {
            (
                host::__WASI_FILETYPE_DIRECTORY,
                host::RIGHTS_DIRECTORY_BASE,
                host::RIGHTS_DIRECTORY_INHERITING,
            )
        } else if ft.is_file() {
            (
                host::__WASI_FILETYPE_REGULAR_FILE,
                host::RIGHTS_REGULAR_FILE_BASE,
                host::RIGHTS_REGULAR_FILE_INHERITING,
            )
        } else if ft.is_socket() {
            use nix::sys::socket;
            match socket::getsockopt(fd.as_raw_fd(), socket::sockopt::SockType)? {
                socket::SockType::Datagram => (
                    host::__WASI_FILETYPE_SOCKET_DGRAM,
                    host::RIGHTS_SOCKET_BASE,
                    host::RIGHTS_SOCKET_INHERITING,
                ),
                socket::SockType::Stream => (
                    host::__WASI_FILETYPE_SOCKET_STREAM,
                    host::RIGHTS_SOCKET_BASE,
                    host::RIGHTS_SOCKET_INHERITING,
                ),
                _ => return Err(Error::EINVAL),
            }
        } else if ft.is_fifo() {
            (
                host::__WASI_FILETYPE_UNKNOWN,
                host::RIGHTS_REGULAR_FILE_BASE,
                host::RIGHTS_REGULAR_FILE_INHERITING,
            )
        } else {
            return Err(Error::EINVAL);
        }
    };

    Ok((file_type, rights_base, rights_inheriting))
}
