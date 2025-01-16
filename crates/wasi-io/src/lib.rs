//! # Wasmtime's wasi-io Implementation
//!
//! This crate provides a Wasmtime host implementation of the WASI 0.2 (aka
//! WASIp2 aka Preview 2) wasi-io package. The host implementation is
//! abstract: it is exposed as a set of traits which other crates provide
//! impls of.
//!
//! The wasi-io package is the foundation which defines how WASI programs
//! interact with the scheduler. It provides the `pollable`, `input-stream`,
//! and `output-stream` Component Model resources, which other packages
//! (including wasi-filesystem, wasi-sockets, wasi-cli, and wasi-http)
//! expose as the standard way to wait for readiness, and asynchronously read
//! and write to streams.
//!
//! This crate is designed to have no unnecessary dependencies and, in
//! particular, compile without `std`.

pub mod bindings;
mod impls;
pub mod poll;
pub mod streams;
mod view;

pub use view::{IoImpl, IoView};

#[doc(no_inline)]
pub use async_trait::async_trait;
