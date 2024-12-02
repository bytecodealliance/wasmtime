//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(missing_docs)]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

pub mod commands;

#[cfg(feature = "run")]
pub(crate) mod common;
