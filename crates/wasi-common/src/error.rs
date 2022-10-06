//! `wasi_common::Error` is now `anyhow::Error`.
//!
//! Snapshots (right now only `wasi_common::snapshots::preview_1`) contains
//! all of the logic for transforming an `Error` into the snapshot's own
//! `Errno`. They may do so by downcasting the error into any of:
//! * `std::io::Error` - these are thrown by `std`, `cap_std`, etc for most of
//! the operations WASI is concerned with.
//! * `wasi_common::ErrorKind` - these are a subset of the Errnos, and are
//! constructed directly by wasi-common or an impl rather than coming from the
//! OS or some library which doesn't know about WASI.
//! * `wiggle::GuestError`
//! * `std::num::TryFromIntError`
//! * `std::str::Utf8Error`
//! and then applying specialized logic to translate each of those into
//! `Errno`s.
//!
//! The `wasi_common::ErrorExt` trait provides human-friendly constructors for
//! the `wasi_common::ErrorKind` variants .
//!
//! If you throw an error that does not downcast to one of those, it will turn
//! into a `wiggle::Trap` and terminate execution.
//!
//! The real value of using `anyhow::Error` here is being able to use
//! `anyhow::Result::context` to aid in debugging of errors.

pub use anyhow::{Context, Error};

/// Internal error type for the `wasi-common` crate.
///
/// This Contains variants of the WASI `$errno` type that are used internally
/// by the crate, and which aren't one-to-one with a `std::io::ErrorKind`
/// error.
///
/// When the Rust [io_error_more] feature is stabilized, that will enable
/// us to replace several more of these codes with `std::io::ErrorKind` codes.
///
/// [io_error_more]: https://doc.rust-lang.org/beta/unstable-book/library-features/io-error-more.html
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Errno::TooBig: Argument list too long
    #[error("TooBig: Argument list too long")]
    TooBig,
    /// Errno::Badf: Bad file descriptor
    #[error("Badf: Bad file descriptor")]
    Badf,
    /// Errno::Ilseq: Illegal byte sequence
    #[error("Ilseq: Illegal byte sequence")]
    Ilseq,
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
    /// Errno::Perm: Permission denied
    #[error("Permission denied")]
    Perm,
}

pub trait ErrorExt {
    fn trap(msg: impl Into<String>) -> Self;
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
    fn trap(msg: impl Into<String>) -> Self {
        anyhow::anyhow!(msg.into())
    }
    fn not_found() -> Self {
        std::io::Error::from(std::io::ErrorKind::NotFound).into()
    }
    fn too_big() -> Self {
        ErrorKind::TooBig.into()
    }
    fn badf() -> Self {
        ErrorKind::Badf.into()
    }
    fn exist() -> Self {
        std::io::Error::from(std::io::ErrorKind::AlreadyExists).into()
    }
    fn illegal_byte_sequence() -> Self {
        ErrorKind::Ilseq.into()
    }
    fn invalid_argument() -> Self {
        std::io::Error::from(std::io::ErrorKind::InvalidInput).into()
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
    fn perm() -> Self {
        ErrorKind::Perm.into()
    }
}
