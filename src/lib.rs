//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(missing_docs)]

pub mod commands;

#[cfg(any(feature = "run", feature = "wizer"))]
pub(crate) mod common;

#[cfg(any(feature = "objdump", all(feature = "hot-blocks", target_os = "linux")))]
pub(crate) mod disas;
