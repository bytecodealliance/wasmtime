#![cfg_attr(docsrs, feature(doc_cfg))]

//! # Wasmtime's WASI Implementation
//!
//! This crate provides a Wasmtime host implementations of different versions of
//! WASI. WASI is implemented with the Rust crate [`tokio`] combined with custom
//! implementations in this crate, and operations are implemented in terms of
//! their native platform equivalents by default.
//!
//! For components and WASIp2, see [`p2`].
//! For WASIp1 and core modules, see the [`p1`] module documentation.
//!
//! For WASIp3, see [`p3`]. WASIp3 support is experimental, unstable and incomplete.

/// The default maximum size, in bytes, that this crate will allocate on the
/// host on a per-read basis.
///
/// This is used throughout this crate as the default limiting factor for
/// buffers allocated on the host for I/O reads, writes, etc. This can be
/// overridden through `WasiCtxBuilder` in some situations, too.
const MAX_READ_SIZE_ALLOC: usize = 64 * 1024;
const DEFAULT_MAX_IO_SIZE: std::num::NonZeroUsize =
    std::num::NonZeroUsize::new(MAX_READ_SIZE_ALLOC).unwrap();

pub mod cli;
pub mod clocks;
mod ctx;
mod error;
pub mod filesystem;
#[cfg(feature = "p1")]
pub mod p0;
#[cfg(feature = "p1")]
pub mod p1;
// FIXME: should gate this module on the `p2` feature but that will require more
// internal refactoring to get that aligned right.
// #[cfg(feature = "p2")]
pub mod p2;
#[cfg(feature = "p3")]
pub mod p3;
pub mod random;
pub mod runtime;
pub mod sockets;
mod view;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub use self::ctx::*;
pub use self::error::{I32Exit, TrappableError};
pub use self::filesystem::{FsPerms, OpenMode};
pub use self::random::{Deterministic, thread_rng};
pub use self::view::*;
#[doc(no_inline)]
pub use async_trait::async_trait;
#[doc(no_inline)]
pub use rand::Rng;
#[doc(no_inline)]
pub use wasmtime::component::{ResourceTable, ResourceTableError};
