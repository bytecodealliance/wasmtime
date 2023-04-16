// Based on:
// https://github.com/bytecodealliance/preview2-prototyping/blob/083879cb955d7cc719eb7fa1b59c6096fcc97bbf/wasi-common/src/error.rs

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
    Badf,
    Exist,
    Inval,
    Noent,
    Overflow,
    Perm,
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
    fn badf() -> Self;
    fn exist() -> Self;
    fn invalid_argument() -> Self;
    fn overflow() -> Self;
    fn perm() -> Self;
}

impl ErrorExt for Error {
    fn not_found() -> Self {
        Errno::Noent.into()
    }
    fn badf() -> Self {
        Errno::Badf.into()
    }
    fn exist() -> Self {
        Errno::Exist.into()
    }
    fn invalid_argument() -> Self {
        Errno::Inval.into()
    }
    fn overflow() -> Self {
        Errno::Overflow.into()
    }
    fn perm() -> Self {
        Errno::Perm.into()
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        match err.kind() {
            std::io::ErrorKind::NotFound => Errno::Noent.into(),
            std::io::ErrorKind::PermissionDenied => Errno::Perm.into(),
            std::io::ErrorKind::AlreadyExists => Errno::Exist.into(),
            std::io::ErrorKind::InvalidInput => Errno::Inval.into(),
            _ => Error::trap(anyhow::anyhow!(err).context("Unknown OS error")),
        }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(_err: std::num::TryFromIntError) -> Error {
        Errno::Overflow.into()
    }
}
