pub use anyhow::Error;

/// Internal error type for the `wasi-common` crate.
/// Contains variants of the WASI `$errno` type are added according to what is actually used internally by
/// the crate. Not all values are represented presently.

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    /// Errno::TooBig: Argument list too long
    #[error("TooBig: Argument list too long")]
    TooBig,
    /// Errno::Badf: Bad file descriptor
    #[error("Badf: Bad file descriptor")]
    Badf,
    /// Errno::Exist: File exists
    #[error("Exist: File exists")]
    Exist,
    /// Errno::Ilseq: Illegal byte sequence
    #[error("Ilseq: Illegal byte sequence")]
    Ilseq,
    /// Errno::Inval: Invalid argument
    #[error("Inval: Invalid argument")]
    Inval,
    /// Errno::Io: I/O error
    #[error("Io: I/O error")]
    Io,
    /// Errno::Nametoolong: Filename too long
    #[error("Nametoolong: Filename too long")]
    Nametoolong,
    /// Errno::Notdir: Not a directory or a symbolic link to a directory.
    #[error("Notdir: Not a directory or a symbolic link to a directory")]
    Notdir,
    /// Errno::Notsup: Not supported, or operation not supported on socket.
    #[error("Notsup: Not supported, or operation not supported on socket")]
    Notsup,
    /// Errno::Overflow: Value too large to be stored in data type.
    #[error("Overflow: Value too large to be stored in data type")]
    Overflow,
    /// Errno::Range: Result too large
    #[error("Range: Result too large")]
    Range,
    /// Errno::Spipe: Invalid seek
    #[error("Spipe: Invalid seek")]
    Spipe,
    /// Errno::NotCapable: Not capable
    #[error("Not capable")]
    NotCapable,
}

pub trait ErrorExt {
    fn trap(msg: impl Into<String>) -> Self;
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
    fn not_capable() -> Self;
}

impl ErrorExt for Error {
    fn trap(msg: impl Into<String>) -> Self {
        anyhow::anyhow!(msg.into())
    }
    fn too_big() -> Self {
        ErrorKind::TooBig.into()
    }
    fn badf() -> Self {
        ErrorKind::Badf.into()
    }
    fn exist() -> Self {
        ErrorKind::Exist.into()
    }
    fn illegal_byte_sequence() -> Self {
        ErrorKind::Ilseq.into()
    }
    fn invalid_argument() -> Self {
        ErrorKind::Inval.into()
    }
    fn io() -> Self {
        ErrorKind::Io.into()
    }
    fn name_too_long() -> Self {
        ErrorKind::Nametoolong.into()
    }
    fn not_dir() -> Self {
        ErrorKind::Notdir.into()
    }
    fn not_supported() -> Self {
        ErrorKind::Notsup.into()
    }
    fn overflow() -> Self {
        ErrorKind::Overflow.into()
    }
    fn range() -> Self {
        ErrorKind::Range.into()
    }
    fn seek_pipe() -> Self {
        ErrorKind::Spipe.into()
    }
    fn not_capable() -> Self {
        ErrorKind::NotCapable.into()
    }
}
