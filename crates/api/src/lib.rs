//! Wasmtime's embedding API
//!
//! This crate contains a high-level API used to interact with WebAssembly
//! modules. The API here is intended to mirror the proposed [WebAssembly C
//! API](https://github.com/WebAssembly/wasm-c-api), with small extensions here
//! and there to implement Rust idioms. This crate also defines the actual C API
//! itself for consumption from other languages.

#![allow(improper_ctypes)]

mod callable;
mod context;
mod externals;
mod instance;
mod module;
mod r#ref;
mod runtime;
mod trampoline;
mod trap;
mod types;
mod values;

pub mod wasm;

pub use crate::callable::Callable;
pub use crate::externals::*;
pub use crate::instance::Instance;
pub use crate::module::Module;
pub use crate::r#ref::{AnyRef, HostInfo, HostRef};
pub use crate::runtime::{Config, Engine, Store};
pub use crate::trap::{FrameInfo, Trap, TrapInfo};
pub use crate::types::*;
pub use crate::values::*;
