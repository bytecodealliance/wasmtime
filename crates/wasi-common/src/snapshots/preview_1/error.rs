pub use super::types::{Errno, Error};

pub trait ErrorExt {
    fn not_found() -> Self;
    fn too_big() -> Self;
    fn badf() -> Self;
    fn exist() -> Self;
    fn illegal_byte_sequence() -> Self;
    fn invalid_argument() -> Self;
    fn io() -> Self;
    fn name_too_long() -> Self;
    fn not_dir() -> Self;
    fn not_supported() -> Self;
    fn overflow() -> Self;
    fn range() -> Self;
    fn seek_pipe() -> Self;
    fn perm() -> Self;
}

impl ErrorExt for Error {
    fn not_found() -> Self {
        Errno::Noent.into()
    }
    fn too_big() -> Self {
        Errno::TooBig.into()
    }
    fn badf() -> Self {
        Errno::Badf.into()
    }
    fn exist() -> Self {
        Errno::Exist.into()
    }
    fn illegal_byte_sequence() -> Self {
        Errno::Ilseq.into()
    }
    fn invalid_argument() -> Self {
        Errno::Inval.into()
    }
    fn io() -> Self {
        Errno::Io.into()
    }
    fn name_too_long() -> Self {
        Errno::Nametoolong.into()
    }
    fn not_dir() -> Self {
        Errno::Notdir.into()
    }
    fn not_supported() -> Self {
        Errno::Notsup.into()
    }
    fn overflow() -> Self {
        Errno::Overflow.into()
    }
    fn range() -> Self {
        Errno::Range.into()
    }
    fn seek_pipe() -> Self {
        Errno::Spipe.into()
    }
    fn perm() -> Self {
        Errno::Perm.into()
    }
}

#[cfg(unix)]
fn from_raw_os_error(err: Option<i32>) -> Option<Error> {
    use rustix::io::Errno as RustixErrno;
    if err.is_none() {
        return None;
    }
    Some(match RustixErrno::from_raw_os_error(err.unwrap()) {
        RustixErrno::AGAIN => Errno::Again.into(),
        RustixErrno::PIPE => Errno::Pipe.into(),
        RustixErrno::PERM => Errno::Perm.into(),
        RustixErrno::NOENT => Errno::Noent.into(),
        RustixErrno::NOMEM => Errno::Nomem.into(),
        RustixErrno::TOOBIG => Errno::TooBig.into(),
        RustixErrno::IO => Errno::Io.into(),
        RustixErrno::BADF => Errno::Badf.into(),
        RustixErrno::BUSY => Errno::Busy.into(),
        RustixErrno::ACCESS => Errno::Acces.into(),
        RustixErrno::FAULT => Errno::Fault.into(),
        RustixErrno::NOTDIR => Errno::Notdir.into(),
        RustixErrno::ISDIR => Errno::Isdir.into(),
        RustixErrno::INVAL => Errno::Inval.into(),
        RustixErrno::EXIST => Errno::Exist.into(),
        RustixErrno::FBIG => Errno::Fbig.into(),
        RustixErrno::NOSPC => Errno::Nospc.into(),
        RustixErrno::SPIPE => Errno::Spipe.into(),
        RustixErrno::MFILE => Errno::Mfile.into(),
        RustixErrno::MLINK => Errno::Mlink.into(),
        RustixErrno::NAMETOOLONG => Errno::Nametoolong.into(),
        RustixErrno::NFILE => Errno::Nfile.into(),
        RustixErrno::NOTEMPTY => Errno::Notempty.into(),
        RustixErrno::LOOP => Errno::Loop.into(),
        RustixErrno::OVERFLOW => Errno::Overflow.into(),
        RustixErrno::ILSEQ => Errno::Ilseq.into(),
        RustixErrno::NOTSUP => Errno::Notsup.into(),
        RustixErrno::ADDRINUSE => Errno::Addrinuse.into(),
        RustixErrno::CANCELED => Errno::Canceled.into(),
        RustixErrno::ADDRNOTAVAIL => Errno::Addrnotavail.into(),
        RustixErrno::AFNOSUPPORT => Errno::Afnosupport.into(),
        RustixErrno::ALREADY => Errno::Already.into(),
        RustixErrno::CONNABORTED => Errno::Connaborted.into(),
        RustixErrno::CONNREFUSED => Errno::Connrefused.into(),
        RustixErrno::CONNRESET => Errno::Connreset.into(),
        RustixErrno::DESTADDRREQ => Errno::Destaddrreq.into(),
        RustixErrno::DQUOT => Errno::Dquot.into(),
        RustixErrno::HOSTUNREACH => Errno::Hostunreach.into(),
        RustixErrno::INPROGRESS => Errno::Inprogress.into(),
        RustixErrno::INTR => Errno::Intr.into(),
        RustixErrno::ISCONN => Errno::Isconn.into(),
        RustixErrno::MSGSIZE => Errno::Msgsize.into(),
        RustixErrno::NETDOWN => Errno::Netdown.into(),
        RustixErrno::NETRESET => Errno::Netreset.into(),
        RustixErrno::NETUNREACH => Errno::Netunreach.into(),
        RustixErrno::NOBUFS => Errno::Nobufs.into(),
        RustixErrno::NOPROTOOPT => Errno::Noprotoopt.into(),
        RustixErrno::NOTCONN => Errno::Notconn.into(),
        RustixErrno::NOTSOCK => Errno::Notsock.into(),
        RustixErrno::PROTONOSUPPORT => Errno::Protonosupport.into(),
        RustixErrno::PROTOTYPE => Errno::Prototype.into(),
        RustixErrno::STALE => Errno::Stale.into(),
        RustixErrno::TIMEDOUT => Errno::Timedout.into(),

        // On some platforms.into(), these have the same value as other errno values.
        #[allow(unreachable_patterns)]
        RustixErrno::WOULDBLOCK => Errno::Again.into(),
        #[allow(unreachable_patterns)]
        RustixErrno::OPNOTSUPP => Errno::Notsup.into(),

        _ => return None,
    })
}
#[cfg(windows)]
fn from_raw_os_error(raw_os_error: Option<i32>) -> Option<Error> {
    use windows_sys::Win32::Foundation;
    use windows_sys::Win32::Networking::WinSock;

    match raw_os_error.map(|code| code as u32) {
        Some(Foundation::ERROR_BAD_ENVIRONMENT) => return Some(Errno::TooBig.into()),
        Some(Foundation::ERROR_FILE_NOT_FOUND) => return Some(Errno::Noent.into()),
        Some(Foundation::ERROR_PATH_NOT_FOUND) => return Some(Errno::Noent.into()),
        Some(Foundation::ERROR_TOO_MANY_OPEN_FILES) => return Some(Errno::Nfile.into()),
        Some(Foundation::ERROR_ACCESS_DENIED) => return Some(Errno::Acces.into()),
        Some(Foundation::ERROR_SHARING_VIOLATION) => return Some(Errno::Acces.into()),
        Some(Foundation::ERROR_PRIVILEGE_NOT_HELD) => return Some(Errno::Perm.into()),
        Some(Foundation::ERROR_INVALID_HANDLE) => return Some(Errno::Badf.into()),
        Some(Foundation::ERROR_INVALID_NAME) => return Some(Errno::Noent.into()),
        Some(Foundation::ERROR_NOT_ENOUGH_MEMORY) => return Some(Errno::Nomem.into()),
        Some(Foundation::ERROR_OUTOFMEMORY) => return Some(Errno::Nomem.into()),
        Some(Foundation::ERROR_DIR_NOT_EMPTY) => return Some(Errno::Notempty.into()),
        Some(Foundation::ERROR_NOT_READY) => return Some(Errno::Busy.into()),
        Some(Foundation::ERROR_BUSY) => return Some(Errno::Busy.into()),
        Some(Foundation::ERROR_NOT_SUPPORTED) => return Some(Errno::Notsup.into()),
        Some(Foundation::ERROR_FILE_EXISTS) => return Some(Errno::Exist.into()),
        Some(Foundation::ERROR_BROKEN_PIPE) => return Some(Errno::Pipe.into()),
        Some(Foundation::ERROR_BUFFER_OVERFLOW) => return Some(Errno::Nametoolong.into()),
        Some(Foundation::ERROR_NOT_A_REPARSE_POINT) => return Some(Errno::Inval.into()),
        Some(Foundation::ERROR_NEGATIVE_SEEK) => return Some(Errno::Inval.into()),
        Some(Foundation::ERROR_DIRECTORY) => return Some(Errno::Notdir.into()),
        Some(Foundation::ERROR_ALREADY_EXISTS) => return Some(Errno::Exist.into()),
        Some(Foundation::ERROR_STOPPED_ON_SYMLINK) => return Some(Errno::Loop.into()),
        Some(Foundation::ERROR_DIRECTORY_NOT_SUPPORTED) => return Some(Errno::Isdir.into()),
        _ => {}
    }

    match raw_os_error {
        Some(WinSock::WSAEWOULDBLOCK) => Some(Errno::Again.into()),
        Some(WinSock::WSAECANCELLED) => Some(Errno::Canceled.into()),
        Some(WinSock::WSA_E_CANCELLED) => Some(Errno::Canceled.into()),
        Some(WinSock::WSAEBADF) => Some(Errno::Badf.into()),
        Some(WinSock::WSAEFAULT) => Some(Errno::Fault.into()),
        Some(WinSock::WSAEINVAL) => Some(Errno::Inval.into()),
        Some(WinSock::WSAEMFILE) => Some(Errno::Mfile.into()),
        Some(WinSock::WSAENAMETOOLONG) => Some(Errno::Nametoolong.into()),
        Some(WinSock::WSAENOTEMPTY) => Some(Errno::Notempty.into()),
        Some(WinSock::WSAELOOP) => Some(Errno::Loop.into()),
        Some(WinSock::WSAEOPNOTSUPP) => Some(Errno::Notsup.into()),
        Some(WinSock::WSAEADDRINUSE) => Some(Errno::Addrinuse.into()),
        Some(WinSock::WSAEACCES) => Some(Errno::Acces.into()),
        Some(WinSock::WSAEADDRNOTAVAIL) => Some(Errno::Addrnotavail.into()),
        Some(WinSock::WSAEAFNOSUPPORT) => Some(Errno::Afnosupport.into()),
        Some(WinSock::WSAEALREADY) => Some(Errno::Already.into()),
        Some(WinSock::WSAECONNABORTED) => Some(Errno::Connaborted.into()),
        Some(WinSock::WSAECONNREFUSED) => Some(Errno::Connrefused.into()),
        Some(WinSock::WSAECONNRESET) => Some(Errno::Connreset.into()),
        Some(WinSock::WSAEDESTADDRREQ) => Some(Errno::Destaddrreq.into()),
        Some(WinSock::WSAEDQUOT) => Some(Errno::Dquot.into()),
        Some(WinSock::WSAEHOSTUNREACH) => Some(Errno::Hostunreach.into()),
        Some(WinSock::WSAEINPROGRESS) => Some(Errno::Inprogress.into()),
        Some(WinSock::WSAEINTR) => Some(Errno::Intr.into()),
        Some(WinSock::WSAEISCONN) => Some(Errno::Isconn.into()),
        Some(WinSock::WSAEMSGSIZE) => Some(Errno::Msgsize.into()),
        Some(WinSock::WSAENETDOWN) => Some(Errno::Netdown.into()),
        Some(WinSock::WSAENETRESET) => Some(Errno::Netreset.into()),
        Some(WinSock::WSAENETUNREACH) => Some(Errno::Netunreach.into()),
        Some(WinSock::WSAENOBUFS) => Some(Errno::Nobufs.into()),
        Some(WinSock::WSAENOPROTOOPT) => Some(Errno::Noprotoopt.into()),
        Some(WinSock::WSAENOTCONN) => Some(Errno::Notconn.into()),
        Some(WinSock::WSAENOTSOCK) => Some(Errno::Notsock.into()),
        Some(WinSock::WSAEPROTONOSUPPORT) => Some(Errno::Protonosupport.into()),
        Some(WinSock::WSAEPROTOTYPE) => Some(Errno::Prototype.into()),
        Some(WinSock::WSAESTALE) => Some(Errno::Stale.into()),
        Some(WinSock::WSAETIMEDOUT) => Some(Errno::Timedout.into()),
        _ => None,
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        match from_raw_os_error(err.raw_os_error()) {
            Some(errno) => errno,
            None => match err.kind() {
                std::io::ErrorKind::NotFound => Errno::Noent.into(),
                std::io::ErrorKind::PermissionDenied => Errno::Perm.into(),
                std::io::ErrorKind::AlreadyExists => Errno::Exist.into(),
                std::io::ErrorKind::InvalidInput => Errno::Inval.into(),
                _ => Error::trap(anyhow::anyhow!(err).context("Unknown OS error")),
            },
        }
    }
}

impl From<cap_rand::Error> for Error {
    fn from(err: cap_rand::Error) -> Error {
        // I picked Error::Io as a 'reasonable default', FIXME dan is this ok?
        from_raw_os_error(err.raw_os_error()).unwrap_or_else(|| Error::from(Errno::Io))
    }
}

impl From<wiggle::GuestError> for Error {
    fn from(err: wiggle::GuestError) -> Error {
        use wiggle::GuestError::*;
        match err {
            InvalidFlagValue { .. } => Errno::Inval.into(),
            InvalidEnumValue { .. } => Errno::Inval.into(),
            // As per
            // https://github.com/WebAssembly/wasi/blob/main/legacy/tools/witx-docs.md#pointers
            //
            // > If a misaligned pointer is passed to a function, the function
            // > shall trap.
            // >
            // > If an out-of-bounds pointer is passed to a function and the
            // > function needs to dereference it, the function shall trap.
            //
            // so this turns OOB and misalignment errors into traps.
            PtrOverflow { .. } | PtrOutOfBounds { .. } | PtrNotAligned { .. } => {
                Error::trap(err.into())
            }
            PtrBorrowed { .. } => Errno::Fault.into(),
            InvalidUtf8 { .. } => Errno::Ilseq.into(),
            TryFromIntError { .. } => Errno::Overflow.into(),
            SliceLengthsDiffer { .. } => Errno::Fault.into(),
            BorrowCheckerOutOfHandles { .. } => Errno::Fault.into(),
            InFunc { err, .. } => Error::from(*err),
        }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(_err: std::num::TryFromIntError) -> Error {
        Errno::Overflow.into()
    }
}
