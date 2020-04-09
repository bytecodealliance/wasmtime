use crate::entry::EntryRights;
use crate::sys::oshandle::{AsFile, OsHandle, OsHandleExt};
use crate::wasi::{types, RightsExt};
use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::io;
use std::mem::ManuallyDrop;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};

#[derive(Debug)]
pub(crate) struct OsFile(Cell<RawHandle>);

impl OsFile {
    /// Consumes `other` taking the ownership of the underlying
    /// `RawHandle` file handle.
    pub(crate) fn update_from(&self, other: Self) {
        let new_handle = other.into_raw_handle();
        let old_handle = self.0.get();
        self.0.set(new_handle);
        // We need to remember to close the old_handle.
        unsafe {
            File::from_raw_handle(old_handle);
        }
    }
    /// Clones `self`.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let handle = self.as_file().try_clone()?;
        Ok(Self(Cell::new(handle.into_raw_handle())))
    }
}

impl Drop for OsFile {
    fn drop(&mut self) {
        unsafe {
            File::from_raw_handle(self.as_raw_handle());
        }
    }
}

impl AsRawHandle for OsFile {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.get()
    }
}

impl FromRawHandle for OsFile {
    unsafe fn from_raw_handle(handle: RawHandle) -> Self {
        Self(Cell::new(handle))
    }
}

impl IntoRawHandle for OsFile {
    fn into_raw_handle(self) -> RawHandle {
        // We need to prevent dropping of the OsFile
        let wrapped = ManuallyDrop::new(self);
        wrapped.0.get()
    }
}

impl AsFile for OsFile {
    fn as_file(&self) -> ManuallyDrop<File> {
        let file = unsafe { File::from_raw_handle(self.0.get()) };
        ManuallyDrop::new(file)
    }
}

impl AsRawHandle for OsHandle {
    fn as_raw_handle(&self) -> RawHandle {
        match self {
            Self::OsFile(file) => file.as_raw_handle(),
            Self::Stdin => io::stdin().as_raw_handle(),
            Self::Stdout => io::stdout().as_raw_handle(),
            Self::Stderr => io::stderr().as_raw_handle(),
        }
    }
}

impl AsFile for OsHandle {
    fn as_file(&self) -> ManuallyDrop<File> {
        let file = unsafe { File::from_raw_handle(self.as_raw_handle()) };
        ManuallyDrop::new(file)
    }
}

impl From<File> for OsHandle {
    fn from(file: File) -> Self {
        Self::from(unsafe { OsFile::from_raw_handle(file.into_raw_handle()) })
    }
}

impl OsHandleExt for OsHandle {
    fn get_file_type(&self) -> io::Result<types::Filetype> {
        let file_type = unsafe { winx::file::get_file_type(self.as_raw_handle())? };
        let file_type = if file_type.is_char() {
            // character file: LPT device or console
            // TODO: rule out LPT device
            types::Filetype::CharacterDevice
        } else if file_type.is_disk() {
            // disk file: file, dir or disk device
            let file = self.as_file();
            let meta = file.metadata()?;
            if meta.is_dir() {
                types::Filetype::Directory
            } else if meta.is_file() {
                types::Filetype::RegularFile
            } else {
                return Err(io::Error::from_raw_os_error(libc::EINVAL));
            }
        } else if file_type.is_pipe() {
            // pipe object: socket, named pipe or anonymous pipe
            // TODO: what about pipes, etc?
            types::Filetype::SocketStream
        } else {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        };
        Ok(file_type)
    }

    fn get_rights(&self, file_type: types::Filetype) -> io::Result<EntryRights> {
        use winx::file::{query_access_information, AccessMode};
        let (base, inheriting) = match file_type {
            types::Filetype::BlockDevice => (
                types::Rights::block_device_base(),
                types::Rights::block_device_inheriting(),
            ),
            types::Filetype::CharacterDevice => {
                (types::Rights::tty_base(), types::Rights::tty_base())
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
        match file_type {
            types::Filetype::Directory | types::Filetype::RegularFile => {
                let mode = query_access_information(self.as_raw_handle())?;
                if mode.contains(AccessMode::FILE_GENERIC_READ) {
                    rights.base |= types::Rights::FD_READ;
                }
                if mode.contains(AccessMode::FILE_GENERIC_WRITE) {
                    rights.base |= types::Rights::FD_WRITE;
                }
            }
            _ => {
                // TODO: is there a way around this? On windows, it seems
                // we cannot check access rights for anything but dirs and regular files
            }
        }
        Ok(rights)
    }

    fn from_null() -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open("NUL")?;
        Ok(Self::from(file))
    }
}
