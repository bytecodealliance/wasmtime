use crate::host;

use std::fs::File;
use std::os::unix::prelude::{FileTypeExt, FromRawFd, IntoRawFd, RawFd, AsRawFd};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FdObject {
    pub ty: host::__wasi_filetype_t,
    pub rawfd: RawFd,
    pub needs_close: bool,
    // TODO: directories
}

#[derive(Clone, Debug)]
pub struct FdEntry {
    pub fd_object: FdObject,
    pub rights_base: host::__wasi_rights_t,
    pub rights_inheriting: host::__wasi_rights_t,
    pub preopen_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct FdMap {
    entries: HashMap<host::__wasi_fd_t, FdEntry>,
}

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            nix::unistd::close(self.rawfd).unwrap_or_else(|e| eprintln!("FdObject::drop(): {}", e));
        }
    }
}

impl FdEntry {
    pub fn from_file(file: File) -> Self {
        unsafe { Self::from_raw_fd(file.into_raw_fd()) }
    }

    pub fn duplicate<F: AsRawFd>(fd: &F) -> Self {
        unsafe { Self::from_raw_fd(nix::unistd::dup(fd.as_raw_fd()).unwrap()) }
    }
}

impl FromRawFd for FdEntry {
    // TODO: make this a different function with error handling, rather than using the trait method
    unsafe fn from_raw_fd(rawfd: RawFd) -> Self {
        let (ty, mut rights_base, rights_inheriting) =
            determine_type_rights(rawfd).expect("can determine file rights");

        use nix::fcntl::{fcntl, OFlag, F_GETFL};
        let flags_bits = fcntl(rawfd, F_GETFL).expect("fcntl succeeds");
        let flags = OFlag::from_bits_truncate(flags_bits);
        let accmode = flags & OFlag::O_ACCMODE;
        if accmode == OFlag::O_RDONLY {
            rights_base &= !host::__WASI_RIGHT_FD_WRITE;
        } else if accmode == OFlag::O_WRONLY {
            rights_base &= !host::__WASI_RIGHT_FD_READ;
        }

        Self {
            fd_object: FdObject {
                ty: ty,
                rawfd,
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }
}

// TODO: can probably make this safe by using fcntl directly rather than going through `File`
pub unsafe fn determine_type_rights(
    rawfd: RawFd,
) -> Result<
    (
        host::__wasi_filetype_t,
        host::__wasi_rights_t,
        host::__wasi_rights_t,
    ),
    host::__wasi_errno_t,
> {
    let (ty, rights_base, rights_inheriting) = {
        let file = File::from_raw_fd(rawfd);
        let ft = file.metadata().unwrap().file_type();
        // we just make a `File` here for convenience; we don't want it to close when it drops
        std::mem::forget(file);
        if ft.is_block_device() {
            (
                host::__WASI_FILETYPE_BLOCK_DEVICE,
                host::RIGHTS_BLOCK_DEVICE_BASE,
                host::RIGHTS_BLOCK_DEVICE_INHERITING,
            )
        } else if ft.is_char_device() {
            if nix::unistd::isatty(rawfd).unwrap() {
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
            match socket::getsockopt(rawfd, socket::sockopt::SockType).unwrap() {
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
                _ => return Err(host::__WASI_EINVAL),
            }
        } else if ft.is_fifo() {
            (
                host::__WASI_FILETYPE_SOCKET_STREAM,
                host::RIGHTS_SOCKET_BASE,
                host::RIGHTS_SOCKET_INHERITING,
            )
        } else {
            return Err(host::__WASI_EINVAL);
        }
    };
    Ok((ty, rights_base, rights_inheriting))
}

impl FdMap {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new()
        }
    }

    pub(crate) fn insert_fd_entry_at(&mut self, fd: host::__wasi_fd_t, fe: FdEntry) {
        self.entries.insert(fd, fe);
    }

    pub(crate) fn get(&self, fd: &host::__wasi_fd_t) -> Option<&FdEntry> {
        self.entries.get(fd)
    }

    pub(crate) fn get_mut(&mut self, fd: &host::__wasi_fd_t) -> Option<&mut FdEntry> {
        self.entries.get_mut(fd)
    }

    pub(crate) fn remove(&mut self, fd: &host::__wasi_fd_t) -> Option<FdEntry> {
        self.entries.remove(fd)
    }

    pub fn get_fd_entry(
        &self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&FdEntry, host::__wasi_errno_t> {
        if let Some(fe) = self.entries.get(&fd) {
            // validate rights
            if !fe.rights_base & rights_base != 0 || !fe.rights_inheriting & rights_inheriting != 0
            {
                Err(host::__WASI_ENOTCAPABLE)
            } else {
                Ok(fe)
            }
        } else {
            Err(host::__WASI_EBADF)
        }
    }

    pub fn insert_fd_entry(
        &mut self,
        fe: FdEntry,
    ) -> Result<host::__wasi_fd_t, host::__wasi_errno_t> {
        // never insert where stdio handles usually are
        let mut fd = 3;
        while self.entries.contains_key(&fd) {
            if let Some(next_fd) = fd.checked_add(1) {
                fd = next_fd;
            } else {
                return Err(host::__WASI_EMFILE);
            }
        }
        self.entries.insert(fd, fe);
        Ok(fd)
    }
}