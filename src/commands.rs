//! The module for the Wasmtime CLI commands.

mod config;
mod run;
mod wasm2obj;
mod wast;

pub use self::{config::*, run::*, wasm2obj::*, wast::*};
