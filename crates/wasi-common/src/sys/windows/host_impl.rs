//! WASI host types specific to Windows host.
use crate::host::FileType;
use crate::wasi::{self, WasiError, WasiResult};
use std::convert::TryInto;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::time::{SystemTime, UNIX_EPOCH};
use winapi::shared::winerror;

impl From<io::Error> for WasiError {
    fn from(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(code) => match code as u32 {
                winerror::ERROR_SUCCESS => Self::ESUCCESS,
                winerror::ERROR_BAD_ENVIRONMENT => Self::E2BIG,
                winerror::ERROR_FILE_NOT_FOUND => Self::ENOENT,
                winerror::ERROR_PATH_NOT_FOUND => Self::ENOENT,
                winerror::ERROR_TOO_MANY_OPEN_FILES => Self::ENFILE,
                winerror::ERROR_ACCESS_DENIED => Self::EACCES,
                winerror::ERROR_SHARING_VIOLATION => Self::EACCES,
                winerror::ERROR_PRIVILEGE_NOT_HELD => Self::ENOTCAPABLE,
                winerror::ERROR_INVALID_HANDLE => Self::EBADF,
                winerror::ERROR_INVALID_NAME => Self::ENOENT,
                winerror::ERROR_NOT_ENOUGH_MEMORY => Self::ENOMEM,
                winerror::ERROR_OUTOFMEMORY => Self::ENOMEM,
                winerror::ERROR_DIR_NOT_EMPTY => Self::ENOTEMPTY,
                winerror::ERROR_NOT_READY => Self::EBUSY,
                winerror::ERROR_BUSY => Self::EBUSY,
                winerror::ERROR_NOT_SUPPORTED => Self::ENOTSUP,
                winerror::ERROR_FILE_EXISTS => Self::EEXIST,
                winerror::ERROR_BROKEN_PIPE => Self::EPIPE,
                winerror::ERROR_BUFFER_OVERFLOW => Self::ENAMETOOLONG,
                winerror::ERROR_NOT_A_REPARSE_POINT => Self::EINVAL,
                winerror::ERROR_NEGATIVE_SEEK => Self::EINVAL,
                winerror::ERROR_DIRECTORY => Self::ENOTDIR,
                winerror::ERROR_ALREADY_EXISTS => Self::EEXIST,
                x => {
                    log::debug!("unknown error value: {}", x);
                    Self::EIO
                }
            },
            None => {
                log::debug!("Other I/O error: {}", err);
                Self::EIO
            }
        }
    }
}

pub(crate) fn filetype_from_std(ftype: &fs::FileType) -> FileType {
    if ftype.is_file() {
        FileType::RegularFile
    } else if ftype.is_dir() {
        FileType::Directory
    } else if ftype.is_symlink() {
        FileType::Symlink
    } else {
        FileType::Unknown
    }
}

fn num_hardlinks(file: &File) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.nNumberOfLinks.into())
}

fn device_id(file: &File) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.dwVolumeSerialNumber.into())
}

pub(crate) fn file_serial_no(file: &File) -> io::Result<u64> {
    let info = winx::file::get_fileinfo(file)?;
    let high = info.nFileIndexHigh;
    let low = info.nFileIndexLow;
    let no = (u64::from(high) << 32) | u64::from(low);
    Ok(no)
}

fn change_time(file: &File) -> io::Result<i64> {
    winx::file::change_time(file)
}

fn systemtime_to_timestamp(st: SystemTime) -> WasiResult<u64> {
    st.duration_since(UNIX_EPOCH)
        .map_err(|_| WasiError::EINVAL)? // date earlier than UNIX_EPOCH
        .as_nanos()
        .try_into()
        .map_err(Into::into) // u128 doesn't fit into u64
}

pub(crate) fn filestat_from_win(file: &File) -> WasiResult<wasi::__wasi_filestat_t> {
    let metadata = file.metadata()?;
    Ok(wasi::__wasi_filestat_t {
        dev: device_id(file)?,
        ino: file_serial_no(file)?,
        nlink: num_hardlinks(file)?.try_into()?, // u64 doesn't fit into u32
        size: metadata.len(),
        atim: systemtime_to_timestamp(metadata.accessed()?)?,
        ctim: change_time(file)?.try_into()?, // i64 doesn't fit into u64
        mtim: systemtime_to_timestamp(metadata.modified()?)?,
        filetype: filetype_from_std(&metadata.file_type()).to_wasi(),
    })
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn path_from_host<S: AsRef<OsStr>>(s: S) -> WasiResult<String> {
    let vec: Vec<u16> = s.as_ref().encode_wide().collect();
    String::from_utf16(&vec).map_err(|_| WasiError::EILSEQ)
}
