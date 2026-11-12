//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(missing_docs)]

// Cargo's `unused_dependencies` lint warns about dependencies that rustc never
// used, but some dependencies here are only used under a combination of
// features (or only exist to enable features of transitive dependencies), which
// can't be expressed in `Cargo.toml`. Fake a use of them here to keep the lint
// quiet.
//
// * `tracing` is only a dependency to support the `disable-logging` feature.
// * `wasmtime-debugger` and `gdbstub-component-artifact` are only used by the
//   `run` and `serve` subcommands.
#[cfg(feature = "disable-logging")]
use tracing as _;
#[cfg(feature = "gdbstub")]
use gdbstub_component_artifact as _;
#[cfg(feature = "debug")]
use wasmtime_debugger as _;

pub mod commands;

#[cfg(any(feature = "cranelift", feature = "winch"))]
pub(crate) mod code_builder;

#[cfg(any(feature = "run", feature = "wizer"))]
pub(crate) mod common;

#[cfg(any(feature = "objdump", all(feature = "hot-blocks", target_os = "linux")))]
pub(crate) mod disas;
