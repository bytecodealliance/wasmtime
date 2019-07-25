//! WASI host types specific to Windows host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
use crate::{host, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub(crate) fn errno_from_win(error: winx::winerror::WinError) -> host::__wasi_errno_t {
    // TODO: implement error mapping between Windows and WASI
    use winx::winerror::WinError::*;
    match error {
        ERROR_SUCCESS => host::__WASI_ESUCCESS,
        ERROR_BAD_ENVIRONMENT => host::__WASI_E2BIG,
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => host::__WASI_ENOENT,
        ERROR_TOO_MANY_OPEN_FILES => host::__WASI_ENFILE,
        ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION => host::__WASI_EACCES,
        ERROR_INVALID_HANDLE | ERROR_INVALID_NAME => host::__WASI_EBADF,
        ERROR_NOT_ENOUGH_MEMORY | ERROR_OUTOFMEMORY => host::__WASI_ENOMEM,
        ERROR_DIR_NOT_EMPTY => host::__WASI_ENOTEMPTY,
        ERROR_DEV_NOT_EXIST => host::__WASI_EINVAL,
        ERROR_NOT_READY | ERROR_BUSY => host::__WASI_EBUSY,
        ERROR_NOT_SUPPORTED => host::__WASI_ENOTSUP,
        ERROR_FILE_EXISTS => host::__WASI_EEXIST,
        ERROR_BROKEN_PIPE => host::__WASI_EPIPE,
        ERROR_BUFFER_OVERFLOW => host::__WASI_ENAMETOOLONG,
        ERROR_DISK_FULL => host::__WASI_ENOSPC,
        ERROR_SHARING_BUFFER_EXCEEDED => host::__WASI_ENFILE,
        _ => host::__WASI_ENOTSUP,
    }
}

pub(crate) fn win_from_fdflags(
    fdflags: host::__wasi_fdflags_t,
) -> (winx::file::AccessRight, winx::file::FlagsAndAttributes) {
    use winx::file::{AccessRight, FlagsAndAttributes};
    // TODO verify this!
    let mut win_rights = AccessRight::empty();
    let mut win_flags_attrs = FlagsAndAttributes::empty();

    if fdflags & host::__WASI_FDFLAG_NONBLOCK != 0 {
        win_flags_attrs.insert(FlagsAndAttributes::FILE_FLAG_OVERLAPPED);
    }
    if fdflags & host::__WASI_FDFLAG_APPEND != 0 {
        win_rights.insert(AccessRight::FILE_APPEND_DATA);
    }
    if fdflags & host::__WASI_FDFLAG_DSYNC != 0
        || fdflags & host::__WASI_FDFLAG_RSYNC != 0
        || fdflags & host::__WASI_FDFLAG_SYNC != 0
    {
        win_rights.insert(AccessRight::SYNCHRONIZE);
    }
    (win_rights, win_flags_attrs)
}

pub(crate) fn fdflags_from_win(rights: winx::file::AccessRight) -> host::__wasi_fdflags_t {
    use winx::file::AccessRight;
    let mut fdflags = 0;
    // TODO verify this!
    if rights.contains(AccessRight::FILE_APPEND_DATA) {
        fdflags |= host::__WASI_FDFLAG_APPEND;
    }
    if rights.contains(AccessRight::SYNCHRONIZE) {
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

pub(crate) fn win_from_oflags(
    oflags: host::__wasi_oflags_t,
) -> (
    winx::file::CreationDisposition,
    winx::file::FlagsAndAttributes,
) {
    use winx::file::{CreationDisposition, FlagsAndAttributes};

    let win_flags_attrs = if oflags & host::__WASI_O_DIRECTORY != 0 {
        FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS
    } else {
        FlagsAndAttributes::FILE_ATTRIBUTE_NORMAL
    };

    let win_disp = if oflags & host::__WASI_O_CREAT != 0 && oflags & host::__WASI_O_EXCL != 0 {
        CreationDisposition::CREATE_NEW
    } else if oflags & host::__WASI_O_CREAT != 0 {
        CreationDisposition::CREATE_ALWAYS
    } else if oflags & host::__WASI_O_TRUNC != 0 {
        CreationDisposition::TRUNCATE_EXISTING
    } else {
        CreationDisposition::OPEN_EXISTING
    };

    (win_disp, win_flags_attrs)
}

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_EILSEQ` error is returned.
pub(crate) fn path_from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    let vec: Vec<u16> = s.as_ref().encode_wide().collect();
    String::from_utf16(&vec).map_err(|_| host::__WASI_EILSEQ)
}
