//! `yanix` stands for Yet Another Nix crate, and, well, it is simply
//! a yet another crate in the spirit of the [nix] crate. As such,
//! this crate is heavily inspired by the original `nix` crate, however,
//! at the same time, it makes certain adjustments and additions here and
//! there that are more tailored towards its main use case which is use
//! in our WASI implementation, [wasi-common].
//!
//! [nix]: https://github.com/nix-rust/nix
//! [wasi-common]: https://github.com/bytecodealliance/wasmtime/tree/master/crates/wasi-common
#![cfg(unix)]

pub mod clock;
pub mod errno;
pub mod file;
pub mod poll;
pub mod socket;
pub mod sys;

pub type Result<T> = std::result::Result<T, Error>;

use errno::Errno;
use std::{ffi, num};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("raw errno received")]
    Errno(#[from] Errno),
    #[error("a nul byte was not found in the expected position")]
    NulError(#[from] ffi::NulError),
    #[error("integral type conversion failed")]
    TryFromIntError(#[from] num::TryFromIntError),
}
