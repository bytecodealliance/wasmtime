use crate::entry::{Descriptor, OsHandleRef};
use crate::wasi::{types, RightsExt};
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, RawHandle};

#[derive(Debug)]
pub(crate) struct OsHandle(File);

impl From<File> for OsHandle {
    fn from(file: File) -> Self {
        Self(file)
    }
}

impl AsRawHandle for OsHandle {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}

impl Deref for OsHandle {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OsHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRawHandle for Descriptor {
    fn as_raw_handle(&self) -> RawHandle {
        match self {
            Self::OsHandle(file) => file.as_raw_handle(),
            Self::VirtualFile(_file) => {
                unimplemented!("virtual as_raw_handle");
            }
            Self::Stdin => io::stdin().as_raw_handle(),
            Self::Stdout => io::stdout().as_raw_handle(),
            Self::Stderr => io::stderr().as_raw_handle(),
        }
    }
}

pub(crate) fn descriptor_as_oshandle<'lifetime>(
    desc: &'lifetime Descriptor,
) -> OsHandleRef<'lifetime> {
    OsHandleRef::new(ManuallyDrop::new(OsHandle::from(unsafe {
        File::from_raw_handle(desc.as_raw_handle())
    })))
}

/// Returns the set of all possible rights that are both relevant for the file
/// type and consistent with the open mode.
///
/// This function is unsafe because it operates on a raw file descriptor.
pub(crate) unsafe fn determine_type_and_access_rights<Handle: AsRawHandle>(
    handle: &Handle,
) -> io::Result<(types::Filetype, types::Rights, types::Rights)> {
    use winx::file::{query_access_information, AccessMode};

    let (file_type, mut rights_base, rights_inheriting) = determine_type_rights(handle)?;

    match file_type {
        types::Filetype::Directory | types::Filetype::RegularFile => {
            let mode = query_access_information(handle.as_raw_handle())?;
            if mode.contains(AccessMode::FILE_GENERIC_READ) {
                rights_base |= types::Rights::FD_READ;
            }
            if mode.contains(AccessMode::FILE_GENERIC_WRITE) {
                rights_base |= types::Rights::FD_WRITE;
            }
        }
        _ => {
            // TODO: is there a way around this? On windows, it seems
            // we cannot check access rights for anything but dirs and regular files
        }
    }

    Ok((file_type, rights_base, rights_inheriting))
}

/// Returns the set of all possible rights that are relevant for file type.
///
/// This function is unsafe because it operates on a raw file descriptor.
pub(crate) unsafe fn determine_type_rights<Handle: AsRawHandle>(
    handle: &Handle,
) -> io::Result<(types::Filetype, types::Rights, types::Rights)> {
    let (file_type, rights_base, rights_inheriting) = {
        let file_type = winx::file::get_file_type(handle.as_raw_handle())?;
        if file_type.is_char() {
            // character file: LPT device or console
            // TODO: rule out LPT device
            (
                types::Filetype::CharacterDevice,
                types::Rights::tty_base(),
                types::Rights::tty_base(),
            )
        } else if file_type.is_disk() {
            // disk file: file, dir or disk device
            let file = std::mem::ManuallyDrop::new(File::from_raw_handle(handle.as_raw_handle()));
            let meta = file.metadata()?;
            if meta.is_dir() {
                (
                    types::Filetype::Directory,
                    types::Rights::directory_base(),
                    types::Rights::directory_inheriting(),
                )
            } else if meta.is_file() {
                (
                    types::Filetype::RegularFile,
                    types::Rights::regular_file_base(),
                    types::Rights::regular_file_inheriting(),
                )
            } else {
                return Err(io::Error::from_raw_os_error(libc::EINVAL));
            }
        } else if file_type.is_pipe() {
            // pipe object: socket, named pipe or anonymous pipe
            // TODO: what about pipes, etc?
            (
                types::Filetype::SocketStream,
                types::Rights::socket_base(),
                types::Rights::socket_inheriting(),
            )
        } else {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }
    };
    Ok((file_type, rights_base, rights_inheriting))
}
