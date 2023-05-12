//! The module for the Wasmtime CLI commands.

mod compile;
mod config;
mod explore;
mod run;
mod settings;
mod wast;

pub use self::{compile::*, config::*, explore::*, run::*, settings::*, wast::*};
