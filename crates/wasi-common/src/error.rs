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

    #[error("IO: {0}")]
    IoError(#[from] std::io::Error),

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
