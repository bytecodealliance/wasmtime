#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

mod clocks;
mod error;
mod filesystem;
mod network;
pub mod p2;
#[cfg(feature = "preview1")]
pub mod preview0;
#[cfg(feature = "preview1")]
pub mod preview1;
mod random;
pub mod runtime;

pub use self::clocks::{HostMonotonicClock, HostWallClock};
pub use self::error::{I32Exit, TrappableError};
pub use self::filesystem::{DirPerms, FilePerms, OpenMode};
pub use self::network::{Network, SocketAddrUse};
pub use self::random::{thread_rng, Deterministic};
#[doc(no_inline)]
pub use async_trait::async_trait;
#[doc(no_inline)]
pub use cap_fs_ext::SystemTimeSpec;
#[doc(no_inline)]
pub use cap_rand::RngCore;
#[doc(no_inline)]
pub use wasmtime::component::{ResourceTable, ResourceTableError};
