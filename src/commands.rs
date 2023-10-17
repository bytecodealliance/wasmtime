//! The module for the Wasmtime CLI commands.

mod compile;
mod config;
mod run;
mod settings;
mod wast;

pub use self::{compile::*, config::*, run::*, settings::*, wast::*};

#[cfg(feature = "serve")]
mod serve;
#[cfg(feature = "serve")]
pub use self::serve::*;

#[cfg(feature = "explore")]
mod explore;
#[cfg(feature = "explore")]
pub use self::explore::*;
