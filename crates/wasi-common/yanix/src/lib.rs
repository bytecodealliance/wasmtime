//! `yanix` stands for Yet Another Nix crate, and, well, it is simply
//! a yet another crate in the spirit of the [nix] crate. As such,
//! this crate is inspired by the original `nix` crate, however,
//! it takes a different approach, using lower-level interfaces with
//! less abstraction, so that it fits better with its main use case
//! which is our WASI implementation, [wasi-common].
//!
//! [nix]: https://github.com/nix-rust/nix
//! [wasi-common]: https://github.com/bytecodealliance/wasmtime/tree/master/crates/wasi-common
#![cfg(unix)]

pub mod clock;
pub mod dir;
pub mod fcntl;
pub mod file;
pub mod poll;
pub mod socket;

mod error;
mod sys;

pub mod fadvise {
    pub use super::sys::fadvise::*;
}

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
