//! WASI host types specific to Windows host.
use crate::{wasi, Error, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub(crate) fn errno_from_win(error: winx::winerror::WinError) -> wasi::__wasi_errno_t {
    // TODO: implement error mapping between Windows and WASI
    use winx::winerror::WinError::*;
    match error {
        ERROR_SUCCESS => wasi::__WASI_ERRNO_SUCCESS,
        ERROR_BAD_ENVIRONMENT => wasi::__WASI_ERRNO_2BIG,
        ERROR_FILE_NOT_FOUND => wasi::__WASI_ERRNO_NOENT,
        ERROR_PATH_NOT_FOUND => wasi::__WASI_ERRNO_NOENT,
        ERROR_TOO_MANY_OPEN_FILES => wasi::__WASI_ERRNO_NFILE,
        ERROR_ACCESS_DENIED => wasi::__WASI_ERRNO_ACCES,
        ERROR_SHARING_VIOLATION => wasi::__WASI_ERRNO_ACCES,
        ERROR_PRIVILEGE_NOT_HELD => wasi::__WASI_ERRNO_NOTCAPABLE, // TODO is this the correct mapping?
        ERROR_INVALID_HANDLE => wasi::__WASI_ERRNO_BADF,
        ERROR_INVALID_NAME => wasi::__WASI_ERRNO_NOENT,
        ERROR_NOT_ENOUGH_MEMORY => wasi::__WASI_ERRNO_NOMEM,
        ERROR_OUTOFMEMORY => wasi::__WASI_ERRNO_NOMEM,
        ERROR_DIR_NOT_EMPTY => wasi::__WASI_ERRNO_NOTEMPTY,
        ERROR_NOT_READY => wasi::__WASI_ERRNO_BUSY,
        ERROR_BUSY => wasi::__WASI_ERRNO_BUSY,
        ERROR_NOT_SUPPORTED => wasi::__WASI_ERRNO_NOTSUP,
        ERROR_FILE_EXISTS => wasi::__WASI_ERRNO_EXIST,
        ERROR_BROKEN_PIPE => wasi::__WASI_ERRNO_PIPE,
        ERROR_BUFFER_OVERFLOW => wasi::__WASI_ERRNO_NAMETOOLONG,
        ERROR_NOT_A_REPARSE_POINT => wasi::__WASI_ERRNO_INVAL,
        ERROR_NEGATIVE_SEEK => wasi::__WASI_ERRNO_INVAL,
        ERROR_DIRECTORY => wasi::__WASI_ERRNO_NOTDIR,
        ERROR_ALREADY_EXISTS => wasi::__WASI_ERRNO_EXIST,
        _ => wasi::__WASI_ERRNO_NOTSUP,
    }
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn path_from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    let vec: Vec<u16> = s.as_ref().encode_wide().collect();
    String::from_utf16(&vec).map_err(|_| Error::EILSEQ)
}
