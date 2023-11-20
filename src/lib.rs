//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(missing_docs)]

pub mod commands;

pub(crate) mod common;

#[cfg(feature = "old-cli")]
pub mod old_cli;
