//! WASI host types specific to Windows host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
use crate::{host, Result};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use winx::file::{AccessMode, Attributes, CreationDisposition, Flags};

pub(crate) fn errno_from_win(error: winx::winerror::WinError) -> host::__wasi_errno_t {
    // TODO: implement error mapping between Windows and WASI
    use winx::winerror::WinError::*;
    match error {
        ERROR_SUCCESS => host::__WASI_ESUCCESS,
        ERROR_BAD_ENVIRONMENT => host::__WASI_E2BIG,
        ERROR_FILE_NOT_FOUND => host::__WASI_ENOENT,
        ERROR_PATH_NOT_FOUND => host::__WASI_ENOENT,
        ERROR_TOO_MANY_OPEN_FILES => host::__WASI_ENFILE,
        ERROR_ACCESS_DENIED => host::__WASI_EACCES,
        ERROR_SHARING_VIOLATION => host::__WASI_EACCES,
        ERROR_PRIVILEGE_NOT_HELD => host::__WASI_ENOTCAPABLE, // TODO is this the correct mapping?
        ERROR_INVALID_HANDLE => host::__WASI_EBADF,
        ERROR_INVALID_NAME => host::__WASI_EINVAL,
        ERROR_NOT_ENOUGH_MEMORY => host::__WASI_ENOMEM,
        ERROR_OUTOFMEMORY => host::__WASI_ENOMEM,
        ERROR_DIR_NOT_EMPTY => host::__WASI_ENOTEMPTY,
        ERROR_NOT_READY => host::__WASI_EBUSY,
        ERROR_BUSY => host::__WASI_EBUSY,
        ERROR_NOT_SUPPORTED => host::__WASI_ENOTSUP,
        ERROR_FILE_EXISTS => host::__WASI_EEXIST,
        ERROR_BROKEN_PIPE => host::__WASI_EPIPE,
        ERROR_BUFFER_OVERFLOW => host::__WASI_ENAMETOOLONG,
        ERROR_NOT_A_REPARSE_POINT => host::__WASI_EINVAL,
        ERROR_NEGATIVE_SEEK => host::__WASI_EINVAL,
        _ => host::__WASI_ENOTSUP,
    }
}

pub(crate) fn fdflags_from_win(mode: AccessMode) -> host::__wasi_fdflags_t {
    let mut fdflags = 0;
    // TODO verify this!
    if mode.contains(AccessMode::FILE_APPEND_DATA) {
        fdflags |= host::__WASI_FDFLAG_APPEND;
    }
    if mode.contains(AccessMode::SYNCHRONIZE) {
        fdflags |= host::__WASI_FDFLAG_DSYNC;
        fdflags |= host::__WASI_FDFLAG_RSYNC;
        fdflags |= host::__WASI_FDFLAG_SYNC;
    }
    // The NONBLOCK equivalent is FILE_FLAG_OVERLAPPED
    // but it seems winapi doesn't provide a mechanism
    // for checking whether the handle supports async IO.
    // On the contrary, I've found some dicsussion online
    // which suggests that on Windows all handles should
    // generally be assumed to be opened with async support
    // and then the program should fallback should that **not**
    // be the case at the time of the operation.
    // TODO: this requires further investigation
    fdflags
}

pub(crate) fn win_from_fdflags(fdflags: host::__wasi_fdflags_t) -> (AccessMode, Flags) {
    let mut access_mode = AccessMode::empty();
    let mut flags = Flags::empty();

    // TODO verify this!
    if fdflags & host::__WASI_FDFLAG_NONBLOCK != 0 {
        flags.insert(Flags::FILE_FLAG_OVERLAPPED);
    }
    if fdflags & host::__WASI_FDFLAG_APPEND != 0 {
        access_mode.insert(AccessMode::FILE_APPEND_DATA);
    }
    if fdflags & host::__WASI_FDFLAG_DSYNC != 0
        || fdflags & host::__WASI_FDFLAG_RSYNC != 0
        || fdflags & host::__WASI_FDFLAG_SYNC != 0
    {
        access_mode.insert(AccessMode::SYNCHRONIZE);
    }

    (access_mode, flags)
}

pub(crate) fn win_from_oflags(oflags: host::__wasi_oflags_t) -> CreationDisposition {
    if oflags & host::__WASI_O_CREAT != 0 {
        if oflags & host::__WASI_O_EXCL != 0 {
            CreationDisposition::CREATE_NEW
        } else {
            CreationDisposition::CREATE_ALWAYS
        }
    } else if oflags & host::__WASI_O_TRUNC != 0 {
        CreationDisposition::TRUNCATE_EXISTING
    } else {
        CreationDisposition::OPEN_EXISTING
    }
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_EILSEQ` error is returned.
pub(crate) fn path_from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    let vec: Vec<u16> = s.as_ref().encode_wide().collect();
    String::from_utf16(&vec).map_err(|_| host::__WASI_EILSEQ)
}
