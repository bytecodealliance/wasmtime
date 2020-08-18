use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Internal error type for the `wasi-common` crate.
/// Contains variants of the WASI `$errno` type are added according to what is actually used internally by
/// the crate. Not all values are represented presently,
#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "Wiggle GuestError: {0}")]
    Guest(#[from] wiggle::GuestError),
    #[error(display = "TryFromIntError: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),
    #[error(display = "Utf8Error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    // Below this, all variants are from the `$errno` type:
    /// Errno::TooBig: Argument list too long
    #[error(display = "TooBig: Argument list too long")]
    TooBig,
    /// Errno::Acces: Permission denied
    #[error(display = "Acces: Permission denied")]
    Acces,
    /// Errno::Badf: Bad file descriptor
    #[error(display = "Badf: Bad file descriptor")]
    Badf,
    /// Errno::Exist: File exists
    #[error(display = "Exist: File exists")]
    Exist,
    /// Errno::Fault: Bad address
    #[error(display = "Fault: Bad address")]
    Fault,
    /// Errno::Fbig: File too large
    #[error(display = "Fbig: File too large")]
    Fbig,
    /// Errno::Ilseq: Illegal byte sequence
    #[error(display = "Ilseq: Illegal byte sequence")]
    Ilseq,
    /// Errno::Inval: Invalid argument
    #[error(display = "Inval: Invalid argument")]
    Inval,
    /// Errno::Io: I/O error
    #[error(display = "Io: I/o error")]
    Io,
    /// Errno::Isdir: Is a directory
    #[error(display = "Isdir: Is a directory")]
    Isdir,
    /// Errno::Loop: Too many levels of symbolic links
    #[error(display = "Loop: Too many levels of symbolic links")]
    Loop,
    /// Errno::Mfile: File descriptor value too large
    #[error(display = "Mfile: File descriptor value too large")]
    Mfile,
    /// Errno::Mlink: Too many links
    #[error(display = "Mlink: Too many links")]
    Mlink,
    /// Errno::Nametoolong: Filename too long
    #[error(display = "Nametoolong: Filename too long")]
    Nametoolong,
    /// Errno::Noent: No such file or directory
    #[error(display = "Noent: No such file or directory")]
    Noent,
    /// Errno::Nospc: No space left on device
    #[error(display = "Nospc: No space left on device")]
    Nospc,
    /// Errno::Notempty: Directory not empty.
    #[error(display = "Notempty: Directory not empty")]
    Notempty,
    /// Errno::Notsup: Not supported, or operation not supported on socket.
    #[error(display = "Notsup: Not supported, or operation not supported on socket")]
    Notsup,
    /// Errno::Overflow: Value too large to be stored in data type.
    #[error(display = "Overflow: Value too large to be stored in data type")]
    Overflow,
    /// Errno::Perm: Operation not permitted
    #[error(display = "Perm: Operation not permitted")]
    Perm,
    /// Errno::Spipe: Invalid seek
    #[error(display = "Spipe: Invalid seek")]
    Spipe,
    /// Errno::Notcapable: Extension: Capabilities insufficient
    #[error(display = "Notcapable: cabailities insufficient")]
    Notcapable,
}

impl From<std::convert::Infallible> for Error {
    fn from(_err: std::convert::Infallible) -> Self {
        unreachable!("should be impossible: From<Infallible>")
    }
}
