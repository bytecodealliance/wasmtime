//! The module for the Wasmtime CLI commands.

mod compile;
mod config;
mod explore;
mod run;
mod settings;
mod wast;

#[cfg(feature = "serve")]
mod serve;

pub use self::{compile::*, config::*, explore::*, run::*, settings::*, wast::*};

#[cfg(feature = "serve")]
pub use self::serve::*;
