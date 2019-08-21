//! Wasmtime embed API. Based on wasm-c-api.

mod callable;
mod context;
mod externals;
mod instance;
mod module;
mod runtime;
mod trampoline;
mod trap;
mod types;
mod values;

#[cfg(feature = "wasm-c-api")]
pub mod wasm;

#[macro_use]
extern crate failure_derive;

pub use crate::callable::Callable;
pub use crate::externals::*;
pub use crate::instance::Instance;
pub use crate::module::Module;
pub use crate::runtime::{Config, Engine, Store};
pub use crate::trap::Trap;
pub use crate::types::*;
pub use crate::values::*;
