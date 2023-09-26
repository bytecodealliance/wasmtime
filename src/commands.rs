//! The module for the Wasmtime CLI commands.

mod compile;
mod config;
mod explore;
mod run;
mod settings;
mod wast;

#[cfg(all(feature = "component-model", feature = "wasi-http"))]
mod serve;

pub use self::{compile::*, config::*, explore::*, run::*, settings::*, wast::*};
