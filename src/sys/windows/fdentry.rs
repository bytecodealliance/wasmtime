use super::host_impl;
use crate::host;

use std::fs::File;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FdObject {
    pub ty: host::__wasi_filetype_t,
    pub raw_handle: RawHandle,
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

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            winx::handle::close(self.raw_handle)
                .unwrap_or_else(|e| eprintln!("FdObject::drop(): {}", e))
        }
    }
}

impl FdEntry {
    pub fn from_file(file: File) -> Self {
        unsafe { Self::from_raw_handle(file.into_raw_handle()) }
    }

    pub fn duplicate<F: AsRawHandle>(fd: &F) -> Self {
        unsafe { Self::from_raw_handle(winx::handle::dup(fd.as_raw_handle()).unwrap()) }
    }
}

impl FromRawHandle for FdEntry {
    unsafe fn from_raw_handle(raw_handle: RawHandle) -> Self {
        use winx::file::{get_file_access_rights, AccessRight};

        let (ty, mut rights_base, rights_inheriting) =
            determine_type_rights(raw_handle).expect("can determine type rights");

        if ty != host::__WASI_FILETYPE_CHARACTER_DEVICE {
            // TODO: is there a way around this? On windows, it seems
            // we cannot check access rights for stdout/in handles
            let rights =
                get_file_access_rights(raw_handle).expect("can determine file access rights");
            let rights = AccessRight::from_bits_truncate(rights);
            if rights.contains(AccessRight::FILE_GENERIC_READ) {
                rights_base |= host::__WASI_RIGHT_FD_READ;
            }
            if rights.contains(AccessRight::FILE_GENERIC_WRITE) {
                rights_base |= host::__WASI_RIGHT_FD_WRITE;
            }
        }

        Self {
            fd_object: FdObject {
                ty,
                raw_handle,
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }
}

pub unsafe fn determine_type_rights(
    raw_handle: RawHandle,
) -> Result<
    (
        host::__wasi_filetype_t,
        host::__wasi_rights_t,
        host::__wasi_rights_t,
    ),
    host::__wasi_errno_t,
> {
    let (ty, rights_base, rights_inheriting) = {
        let file_type = winx::file::get_file_type(raw_handle).map_err(host_impl::errno_from_win)?;
        if file_type.is_char() {
            // character file: LPT device or console
            // TODO: rule out LPT device
            (
                host::__WASI_FILETYPE_CHARACTER_DEVICE,
                host::RIGHTS_TTY_BASE,
                host::RIGHTS_TTY_BASE,
            )
        } else if file_type.is_disk() {
            // disk file: file, dir or disk device
            let file = std::mem::ManuallyDrop::new(File::from_raw_handle(raw_handle));
            let meta = file.metadata().map_err(|_| host::__WASI_EINVAL)?;
            if meta.is_dir() {
                (
                    host::__WASI_FILETYPE_DIRECTORY,
                    host::RIGHTS_DIRECTORY_BASE,
                    host::RIGHTS_DIRECTORY_INHERITING,
                )
            } else if meta.is_file() {
                (
                    host::__WASI_FILETYPE_REGULAR_FILE,
                    host::RIGHTS_REGULAR_FILE_BASE,
                    host::RIGHTS_REGULAR_FILE_INHERITING,
                )
            } else {
                return Err(host::__WASI_EINVAL);
            }
        } else if file_type.is_pipe() {
            // pipe object: socket, named pipe or anonymous pipe
            // TODO: what about pipes, etc?
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
