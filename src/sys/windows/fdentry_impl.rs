use super::host_impl;
use crate::fdentry::Descriptor;
use crate::{host, Result};
use std::fs::File;
use std::io;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, RawHandle};

impl AsRawHandle for Descriptor {
    fn as_raw_handle(&self) -> RawHandle {
        match self {
            Descriptor::File(f) => f.as_raw_handle(),
            Descriptor::Stdin => io::stdin().as_raw_handle(),
            Descriptor::Stdout => io::stdout().as_raw_handle(),
            Descriptor::Stderr => io::stderr().as_raw_handle(),
        }
    }
}

pub(crate) fn determine_type_and_access_rights<Handle: AsRawHandle>(
    handle: &Handle,
) -> Result<(
    host::__wasi_filetype_t,
    host::__wasi_rights_t,
    host::__wasi_rights_t,
)> {
    use winx::file::{get_file_access_rights, AccessRight};

    let (file_type, mut rights_base, rights_inheriting) = determine_type_rights(handle)?;

    match file_type {
        host::__WASI_FILETYPE_DIRECTORY | host::__WASI_FILETYPE_REGULAR_FILE => {
            let rights = get_file_access_rights(handle.as_raw_handle())
                .map_err(host_impl::errno_from_win)?;
            let rights = AccessRight::from_bits_truncate(rights);
            if rights.contains(AccessRight::FILE_GENERIC_READ) {
                rights_base |= host::__WASI_RIGHT_FD_READ;
            }
            if rights.contains(AccessRight::FILE_GENERIC_WRITE) {
                rights_base |= host::__WASI_RIGHT_FD_WRITE;
            }
        }
        _ => {
            // TODO: is there a way around this? On windows, it seems
            // we cannot check access rights for anything but dirs and regular files
        }
    }

    Ok((file_type, rights_base, rights_inheriting))
}

pub(crate) fn determine_type_rights<Handle: AsRawHandle>(
    handle: &Handle,
) -> Result<(
    host::__wasi_filetype_t,
    host::__wasi_rights_t,
    host::__wasi_rights_t,
)> {
    let (file_type, rights_base, rights_inheriting) = {
        let file_type =
            winx::file::get_file_type(handle.as_raw_handle()).map_err(host_impl::errno_from_win)?;
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
            let file = std::mem::ManuallyDrop::new(unsafe {
                File::from_raw_handle(handle.as_raw_handle())
            });
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
    Ok((file_type, rights_base, rights_inheriting))
}
