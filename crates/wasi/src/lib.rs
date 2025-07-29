#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! # Wasmtime's WASI Implementation
//!
//! This crate provides a Wasmtime host implementations of different versions of WASI.
//! WASI is implemented with the Rust crates [`tokio`] and [`cap-std`](cap_std) primarily, meaning that
//! operations are implemented in terms of their native platform equivalents by
//! default.
//!
//! For components and WASIp2, see [`p2`].
//! For WASIp1 and core modules, see the [`preview1`] module documentation.
//!
//! For WASIp3, see [`p3`]. WASIp3 support is experimental, unstable and incomplete.

pub mod cli;
pub mod clocks;
mod ctx;
mod error;
mod filesystem;
pub mod p2;
#[cfg(feature = "p3")]
pub mod p3;
#[cfg(feature = "preview1")]
pub mod preview0;
#[cfg(feature = "preview1")]
pub mod preview1;
pub mod random;
pub mod runtime;
pub mod sockets;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub(crate) use self::ctx::WasiCtxBuilder;
pub use self::error::{I32Exit, TrappableError};
pub use self::filesystem::{DirPerms, FilePerms, OpenMode};
pub use self::random::{Deterministic, thread_rng};
pub use self::sockets::{AllowedNetworkUses, SocketAddrUse};
#[doc(no_inline)]
pub use async_trait::async_trait;
#[doc(no_inline)]
pub use cap_fs_ext::SystemTimeSpec;
#[doc(no_inline)]
pub use cap_rand::RngCore;
#[doc(no_inline)]
pub use wasmtime::component::{ResourceTable, ResourceTableError};
