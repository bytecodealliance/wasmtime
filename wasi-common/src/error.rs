//! wasi-common uses an [`Error`] type which represents either a preview 1 [`Errno`] enum, on
//! [`anyhow::Error`] for trapping execution.
//!
//! The user can construct an [`Error`] out of an [`Errno`] using the `From`/`Into` traits.
//! They may also use [`Error::trap`] to construct an error that traps execution. The contents
//! can be inspected with [`Error::downcast`] and [`Error::downcast_ref`]. Additional context
//! can be provided with the [`Error::context`] method. This context is only observable with the
//! `Display` and `Debug` impls of the error.

use std::fmt;

/// An error returned from the `proc_exit` host syscall.
///
/// Embedders can test if an error returned from wasm is this error, in which
/// case it may signal a non-fatal trap.
#[derive(Debug)]
pub struct I32Exit(pub i32);

impl fmt::Display for I32Exit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Exited with i32 exit status {}", self.0)
    }
}

impl std::error::Error for I32Exit {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errno {
    Success,
    TooBig,
    Acces,
    Addrinuse,
    Addrnotavail,
    Afnosupport,
    Again,
    Already,
    Badf,
    Badmsg,
    Busy,
    Canceled,
    Connaborted,
    Connrefused,
    Connreset,
    Deadlk,
    Destaddrreq,
    Dquot,
    Exist,
    Fault,
    Fbig,
    Hostunreach,
    Idrm,
    Ilseq,
    Inprogress,
    Intr,
    Inval,
    Io,
    Isconn,
    Isdir,
    Loop,
    Mfile,
    Mlink,
    Msgsize,
    Multihop,
    Nametoolong,
    Netdown,
    Netreset,
    Netunreach,
    Nfile,
    Nobufs,
    Nodev,
    Noent,
    Noexec,
    Nolck,
    Nolink,
    Nomem,
    Nomsg,
    Noprotoopt,
    Nospc,
    Nosys,
    Notconn,
    Notdir,
    Notempty,
    Notrecoverable,
    Notsock,
    Notsup,
    Notty,
    Nxio,
    Overflow,
    Ownerdead,
    Perm,
    Pipe,
    Proto,
    Protonosupport,
    Prototype,
    Range,
    Rofs,
    Spipe,
    Srch,
    Stale,
    Timedout,
    Txtbsy,
    Xdev,
}

impl std::fmt::Display for Errno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

impl std::error::Error for Errno {}

#[derive(Debug)]
pub struct Error {
    inner: anyhow::Error,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

impl Error {
    pub fn trap(inner: anyhow::Error) -> Error {
        Self { inner }
    }
    pub fn into(self) -> anyhow::Error {
        self.inner
    }
    pub fn downcast(self) -> Result<Errno, anyhow::Error> {
        self.inner.downcast()
    }
    pub fn downcast_ref(&self) -> Option<&Errno> {
        self.inner.downcast_ref()
    }
    pub fn context(self, s: impl Into<String>) -> Self {
        Self {
            inner: self.inner.context(s.into()),
        }
    }
}

impl From<Errno> for Error {
    fn from(abi: Errno) -> Error {
        Error {
            inner: anyhow::Error::from(abi),
        }
    }
}

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
    fn destination_address_required() -> Self;
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
    fn destination_address_required() -> Self {
        Errno::Destaddrreq.into()
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

impl From<std::num::TryFromIntError> for Error {
    fn from(_err: std::num::TryFromIntError) -> Error {
        Errno::Overflow.into()
    }
}
