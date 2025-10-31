//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(missing_docs)]

pub mod commands;

#[cfg(any(feature = "run", feature = "wizer"))]
pub(crate) mod common;
