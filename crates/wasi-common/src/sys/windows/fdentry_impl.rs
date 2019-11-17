use crate::fdentry::Descriptor;
use crate::{wasi, Error, Result};
use std::fs::File;
use std::io;
use std::ops::{Deref, DerefMut};
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, RawHandle};

#[derive(Debug)]
pub(crate) struct OsFile(File);

impl From<File> for OsFile {
    fn from(file: File) -> Self {
        Self(file)
    }
}

impl AsRawHandle for OsFile {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}

impl Deref for OsFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OsFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRawHandle for Descriptor {
    fn as_raw_handle(&self) -> RawHandle {
        match self {
            Self::OsFile(file) => file.as_raw_handle(),
            Self::Stdin => io::stdin().as_raw_handle(),
            Self::Stdout => io::stdout().as_raw_handle(),
            Self::Stderr => io::stderr().as_raw_handle(),
        }
    }
}

/// This function is unsafe because it operates on a raw file handle.
pub(crate) unsafe fn determine_type_and_access_rights<Handle: AsRawHandle>(
    handle: &Handle,
) -> Result<(
    wasi::__wasi_filetype_t,
    wasi::__wasi_rights_t,
    wasi::__wasi_rights_t,
)> {
    use winx::file::{get_file_access_mode, AccessMode};

    let (file_type, mut rights_base, rights_inheriting) = determine_type_rights(handle)?;

    match file_type {
        wasi::__WASI_FILETYPE_DIRECTORY | wasi::__WASI_FILETYPE_REGULAR_FILE => {
            let mode = get_file_access_mode(handle.as_raw_handle())?;
            if mode.contains(AccessMode::FILE_GENERIC_READ) {
                rights_base |= wasi::__WASI_RIGHTS_FD_READ;
            }
            if mode.contains(AccessMode::FILE_GENERIC_WRITE) {
                rights_base |= wasi::__WASI_RIGHTS_FD_WRITE;
            }
        }
        _ => {
            // TODO: is there a way around this? On windows, it seems
            // we cannot check access rights for anything but dirs and regular files
        }
    }

    Ok((file_type, rights_base, rights_inheriting))
}

/// This function is unsafe because it operates on a raw file handle.
pub(crate) unsafe fn determine_type_rights<Handle: AsRawHandle>(
    handle: &Handle,
) -> Result<(
    wasi::__wasi_filetype_t,
    wasi::__wasi_rights_t,
    wasi::__wasi_rights_t,
)> {
    let (file_type, rights_base, rights_inheriting) = {
        let file_type = winx::file::get_file_type(handle.as_raw_handle())?;
        if file_type.is_char() {
            // character file: LPT device or console
            // TODO: rule out LPT device
            (
                wasi::__WASI_FILETYPE_CHARACTER_DEVICE,
                wasi::RIGHTS_TTY_BASE,
                wasi::RIGHTS_TTY_BASE,
            )
        } else if file_type.is_disk() {
            // disk file: file, dir or disk device
            let file = std::mem::ManuallyDrop::new(File::from_raw_handle(handle.as_raw_handle()));
            let meta = file.metadata().map_err(|_| Error::EINVAL)?;
            if meta.is_dir() {
                (
                    wasi::__WASI_FILETYPE_DIRECTORY,
                    wasi::RIGHTS_DIRECTORY_BASE,
                    wasi::RIGHTS_DIRECTORY_INHERITING,
                )
            } else if meta.is_file() {
                (
                    wasi::__WASI_FILETYPE_REGULAR_FILE,
                    wasi::RIGHTS_REGULAR_FILE_BASE,
                    wasi::RIGHTS_REGULAR_FILE_INHERITING,
                )
            } else {
                return Err(Error::EINVAL);
            }
        } else if file_type.is_pipe() {
            // pipe object: socket, named pipe or anonymous pipe
            // TODO: what about pipes, etc?
            (
                wasi::__WASI_FILETYPE_SOCKET_STREAM,
                wasi::RIGHTS_SOCKET_BASE,
                wasi::RIGHTS_SOCKET_INHERITING,
            )
        } else {
            return Err(Error::EINVAL);
        }
    };
    Ok((file_type, rights_base, rights_inheriting))
}
