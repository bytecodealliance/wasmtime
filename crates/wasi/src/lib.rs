//! # Wasmtime's WASI Implementation
//!
//! This crate provides a Wasmtime host implementation of WASI 0.2 (aka
//! Preview 2), and a compatibility shim that provides an implementation of
//! WASI 0.1 (aka Preview 1).
//!
//!

pub mod bindings;
mod clocks;
pub mod command;
mod ctx;
mod error;
mod filesystem;
mod host;
mod ip_name_lookup;
mod network;
#[cfg(feature = "preview1")]
mod p1ctx;
pub mod pipe;
mod poll;
#[cfg(feature = "preview1")]
pub mod preview0;
#[cfg(feature = "preview1")]
pub mod preview1;
mod random;
pub mod runtime;
mod stdio;
mod stream;
mod tcp;
mod udp;
mod write_stream;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub use self::ctx::{WasiCtx, WasiCtxBuilder, WasiView};
pub use self::error::{I32Exit, TrappableError};
pub use self::filesystem::{DirPerms, FilePerms, FsError, FsResult};
pub use self::network::{Network, SocketAddrUse, SocketError, SocketResult};
#[cfg(feature = "preview1")]
pub use self::p1ctx::WasiP1Ctx;
pub use self::poll::{subscribe, ClosureFuture, MakeFuture, Pollable, PollableFuture, Subscribe};
pub use self::random::{thread_rng, Deterministic};
pub use self::stdio::{
    stderr, stdin, stdout, AsyncStdinStream, AsyncStdoutStream, IsATTY, Stderr, Stdin, StdinStream,
    Stdout, StdoutStream,
};
pub use self::stream::{
    HostInputStream, HostOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};
pub use cap_fs_ext::SystemTimeSpec;
pub use cap_rand::RngCore;
pub use wasmtime::component::{ResourceTable, ResourceTableError};
