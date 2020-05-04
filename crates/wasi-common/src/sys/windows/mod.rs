pub(crate) mod clock;
pub(crate) mod fd;
pub(crate) mod osdir;
pub(crate) mod osfile;
pub(crate) mod oshandle;
pub(crate) mod osother;
pub(crate) mod path;
pub(crate) mod poll;
pub(crate) mod stdio;

use crate::handle::HandleRights;
use crate::sys::AsFile;
use crate::wasi::{types, Errno, Result, RightsExt};
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::mem::ManuallyDrop;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{io, string};
use winapi::shared::winerror;
use winx::file::{CreationDisposition, Flags};

impl<T: AsRawHandle> AsFile for T {
    fn as_file(&self) -> io::Result<ManuallyDrop<File>> {
        let file = unsafe { File::from_raw_handle(self.as_raw_handle()) };
        Ok(ManuallyDrop::new(file))
    }
}

pub(super) fn get_file_type(file: &File) -> io::Result<types::Filetype> {
    let file_type = unsafe { winx::file::get_file_type(file.as_raw_handle())? };
    let file_type = if file_type.is_char() {
        // character file: LPT device or console
        // TODO: rule out LPT device
        types::Filetype::CharacterDevice
    } else if file_type.is_disk() {
        // disk file: file, dir or disk device
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

pub(super) fn get_rights(file_type: &types::Filetype) -> io::Result<HandleRights> {
    let (base, inheriting) = match file_type {
        types::Filetype::BlockDevice => (
            types::Rights::block_device_base(),
            types::Rights::block_device_inheriting(),
        ),
        types::Filetype::CharacterDevice => (types::Rights::tty_base(), types::Rights::tty_base()),
        types::Filetype::SocketDgram | types::Filetype::SocketStream => (
            types::Rights::socket_base(),
            types::Rights::socket_inheriting(),
        ),
        types::Filetype::SymbolicLink | types::Filetype::Unknown => (
            types::Rights::regular_file_base(),
            types::Rights::regular_file_inheriting(),
        ),
        types::Filetype::Directory => (
            types::Rights::directory_base(),
            types::Rights::directory_inheriting(),
        ),
        types::Filetype::RegularFile => (
            types::Rights::regular_file_base(),
            types::Rights::regular_file_inheriting(),
        ),
    };
    let rights = HandleRights::new(base, inheriting);
    Ok(rights)
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> io::Result<File> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;

    // To open a directory using CreateFile, specify the
    // FILE_FLAG_BACKUP_SEMANTICS flag as part of dwFileFlags...
    // cf. https://docs.microsoft.com/en-us/windows/desktop/api/fileapi/nf-fileapi-createfile2
    OpenOptions::new()
        .create(false)
        .write(true)
        .read(true)
        .attributes(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

pub(crate) fn file_serial_no(file: &File) -> io::Result<u64> {
    let info = winx::file::get_fileinfo(file)?;
    let high = info.nFileIndexHigh;
    let low = info.nFileIndexLow;
    let no = (u64::from(high) << 32) | u64::from(low);
    Ok(no)
}

impl From<io::Error> for Errno {
    fn from(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(code) => match code as u32 {
                winerror::ERROR_SUCCESS => Self::Success,
                winerror::ERROR_BAD_ENVIRONMENT => Self::TooBig,
                winerror::ERROR_FILE_NOT_FOUND => Self::Noent,
                winerror::ERROR_PATH_NOT_FOUND => Self::Noent,
                winerror::ERROR_TOO_MANY_OPEN_FILES => Self::Nfile,
                winerror::ERROR_ACCESS_DENIED => Self::Acces,
                winerror::ERROR_SHARING_VIOLATION => Self::Acces,
                winerror::ERROR_PRIVILEGE_NOT_HELD => Self::Notcapable,
                winerror::ERROR_INVALID_HANDLE => Self::Badf,
                winerror::ERROR_INVALID_NAME => Self::Noent,
                winerror::ERROR_NOT_ENOUGH_MEMORY => Self::Nomem,
                winerror::ERROR_OUTOFMEMORY => Self::Nomem,
                winerror::ERROR_DIR_NOT_EMPTY => Self::Notempty,
                winerror::ERROR_NOT_READY => Self::Busy,
                winerror::ERROR_BUSY => Self::Busy,
                winerror::ERROR_NOT_SUPPORTED => Self::Notsup,
                winerror::ERROR_FILE_EXISTS => Self::Exist,
                winerror::ERROR_BROKEN_PIPE => Self::Pipe,
                winerror::ERROR_BUFFER_OVERFLOW => Self::Nametoolong,
                winerror::ERROR_NOT_A_REPARSE_POINT => Self::Inval,
                winerror::ERROR_NEGATIVE_SEEK => Self::Inval,
                winerror::ERROR_DIRECTORY => Self::Notdir,
                winerror::ERROR_ALREADY_EXISTS => Self::Exist,
                x => {
                    log::debug!("winerror: unknown error value: {}", x);
                    Self::Io
                }
            },
            None => {
                log::debug!("Other I/O error: {}", err);
                Self::Io
            }
        }
    }
}

impl From<string::FromUtf16Error> for Errno {
    fn from(_err: string::FromUtf16Error) -> Self {
        Self::Ilseq
    }
}

fn num_hardlinks(file: &File) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.nNumberOfLinks.into())
}

fn device_id(file: &File) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.dwVolumeSerialNumber.into())
}

fn change_time(file: &File) -> io::Result<i64> {
    winx::file::change_time(file)
}

fn systemtime_to_timestamp(st: SystemTime) -> Result<u64> {
    st.duration_since(UNIX_EPOCH)
        .map_err(|_| Errno::Inval)? // date earlier than UNIX_EPOCH
        .as_nanos()
        .try_into()
        .map_err(Into::into) // u128 doesn't fit into u64
}

impl TryFrom<&File> for types::Filestat {
    type Error = Errno;

    fn try_from(file: &File) -> Result<Self> {
        let metadata = file.metadata()?;
        Ok(types::Filestat {
            dev: device_id(file)?,
            ino: file_serial_no(file)?,
            nlink: num_hardlinks(file)?.try_into()?, // u64 doesn't fit into u32
            size: metadata.len(),
            atim: systemtime_to_timestamp(metadata.accessed()?)?,
            ctim: change_time(file)?.try_into()?, // i64 doesn't fit into u64
            mtim: systemtime_to_timestamp(metadata.modified()?)?,
            filetype: metadata.file_type().into(),
        })
    }
}

impl From<types::Oflags> for CreationDisposition {
    fn from(oflags: types::Oflags) -> Self {
        if oflags.contains(&types::Oflags::CREAT) {
            if oflags.contains(&types::Oflags::EXCL) {
                CreationDisposition::CREATE_NEW
            } else {
                CreationDisposition::CREATE_ALWAYS
            }
        } else if oflags.contains(&types::Oflags::TRUNC) {
            CreationDisposition::TRUNCATE_EXISTING
        } else {
            CreationDisposition::OPEN_EXISTING
        }
    }
}

impl From<types::Fdflags> for Flags {
    fn from(fdflags: types::Fdflags) -> Self {
        // Enable backup semantics so directories can be opened as files
        let mut flags = Flags::FILE_FLAG_BACKUP_SEMANTICS;

        // Note: __WASI_FDFLAGS_NONBLOCK is purposely being ignored for files
        // While Windows does inherently support a non-blocking mode on files, the WASI API will
        // treat I/O operations on files as synchronous. WASI might have an async-io API in the future.

        // Technically, Windows only supports __WASI_FDFLAGS_SYNC, but treat all the flags as the same.
        if fdflags.contains(&types::Fdflags::DSYNC)
            || fdflags.contains(&types::Fdflags::RSYNC)
            || fdflags.contains(&types::Fdflags::SYNC)
        {
            flags.insert(Flags::FILE_FLAG_WRITE_THROUGH);
        }

        flags
    }
}
