use cfg_if::cfg_if;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Internal error type for the `wasi-common` crate.
/// Contains variants of the WASI `$errno` type are added according to what is actually used internally by
/// the crate. Not all values are represented presently.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Wiggle GuestError: {0}")]
    Guest(#[from] wiggle::GuestError),
    #[error("TryFromIntError: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),
    #[error("Utf8Error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// The host OS may return an io error that doesn't match one of the
    /// wasi errno variants we expect. We do not expose the details of this
    /// error to the user.
    #[error("Unexpected IoError: {0}")]
    UnexpectedIo(#[source] std::io::Error),

    // Below this, all variants are from the `$errno` type:
    /// Errno::TooBig: Argument list too long
    #[error("TooBig: Argument list too long")]
    TooBig,
    /// Errno::Acces: Permission denied
    #[error("Acces: Permission denied")]
    Acces,
    /// Errno::Badf: Bad file descriptor
    #[error("Badf: Bad file descriptor")]
    Badf,
    /// Errno::Exist: File exists
    #[error("Exist: File exists")]
    Exist,
    /// Errno::Fault: Bad address
    #[error("Fault: Bad address")]
    Fault,
    /// Errno::Fbig: File too large
    #[error("Fbig: File too large")]
    Fbig,
    /// Errno::Ilseq: Illegal byte sequence
    #[error("Ilseq: Illegal byte sequence")]
    Ilseq,
    /// Errno::Inval: Invalid argument
    #[error("Inval: Invalid argument")]
    Inval,
    /// Errno::Io: I/O error
    #[error("Io: I/o error")]
    Io,
    /// Errno::Isdir: Is a directory
    #[error("Isdir: Is a directory")]
    Isdir,
    /// Errno::Loop: Too many levels of symbolic links
    #[error("Loop: Too many levels of symbolic links")]
    Loop,
    /// Errno::Mfile: File descriptor value too large
    #[error("Mfile: File descriptor value too large")]
    Mfile,
    /// Errno::Mlink: Too many links
    #[error("Mlink: Too many links")]
    Mlink,
    /// Errno::Nametoolong: Filename too long
    #[error("Nametoolong: Filename too long")]
    Nametoolong,
    /// Errno::Noent: No such file or directory
    #[error("Noent: No such file or directory")]
    Noent,
    /// Errno::Nospc: No space left on device
    #[error("Nospc: No space left on device")]
    Nospc,
    /// Errno::Notdir: Not a directory or a symbolic link to a directory.
    #[error("Notdir: Not a directory or a symbolic link to a directory")]
    Notdir,
    /// Errno::Notempty: Directory not empty.
    #[error("Notempty: Directory not empty")]
    Notempty,
    /// Errno::Notsup: Not supported, or operation not supported on socket.
    #[error("Notsup: Not supported, or operation not supported on socket")]
    Notsup,
    /// Errno::Overflow: Value too large to be stored in data type.
    #[error("Overflow: Value too large to be stored in data type")]
    Overflow,
    /// Errno::Perm: Operation not permitted
    #[error("Perm: Operation not permitted")]
    Perm,
    /// Errno::Spipe: Invalid seek
    #[error("Spipe: Invalid seek")]
    Spipe,
    /// Errno::Notcapable: Extension: Capabilities insufficient
    #[error("Notcapable: cabailities insufficient")]
    Notcapable,
}

impl From<std::convert::Infallible> for Error {
    fn from(_err: std::convert::Infallible) -> Self {
        unreachable!("should be impossible: From<Infallible>")
    }
}

// Turning an io::Error into an Error has platform-specific behavior
cfg_if! {
    if #[cfg(windows)] {
use winapi::shared::winerror;
use std::io;
impl From<io::Error> for Error {
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
                _ => Self::UnexpectedIo(err),
            },
            None => Self::UnexpectedIo(err),
        }
    }
}

    } else {
use std::io;
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(code) => match code {
                libc::EPERM => Self::Perm,
                libc::ENOENT => Self::Noent,
                libc::E2BIG => Self::TooBig,
                libc::EIO => Self::Io,
                libc::EBADF => Self::Badf,
                libc::EACCES => Self::Acces,
                libc::EFAULT => Self::Fault,
                libc::ENOTDIR => Self::Notdir,
                libc::EISDIR => Self::Isdir,
                libc::EINVAL => Self::Inval,
                libc::EEXIST => Self::Exist,
                libc::EFBIG => Self::Fbig,
                libc::ENOSPC => Self::Nospc,
                libc::ESPIPE => Self::Spipe,
                libc::EMFILE => Self::Mfile,
                libc::EMLINK => Self::Mlink,
                libc::ENAMETOOLONG => Self::Nametoolong,
                libc::ENOTEMPTY => Self::Notempty,
                libc::ELOOP => Self::Loop,
                libc::EOVERFLOW => Self::Overflow,
                libc::EILSEQ => Self::Ilseq,
                libc::ENOTSUP => Self::Notsup,
                _ => Self::UnexpectedIo(err),
            },
            None => {
                Self::UnexpectedIo(err)
            }
        }
    }
}
    }
}
