//! wasi-common uses an [`Error`] type which represents either a preview 1 [`Errno`] enum, on
//! [`anyhow::Error`] for trapping execution.
//!
//! The user can construct an [`Error`] out of an [`Errno`] using the `From`/`Into` traits.
//! They may also use [`Error::trap`] to construct an error that traps execution. The contents
//! can be inspected with [`Error::downcast`] and [`Error::downcast_ref`]. Additional context
//! can be provided with the [`Error::context`] method. This context is only observable with the
//! `Display` and `Debug` impls of the error.

pub use crate::snapshots::preview_1::error::{Error, ErrorExt};
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
