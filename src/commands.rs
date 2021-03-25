//! The module for the Wasmtime CLI commands.

mod compile;
mod config;
mod run;
mod wasm2obj;
mod wast;

pub use self::{compile::*, config::*, run::*, wasm2obj::*, wast::*};
