//! The module for the Wasmtime CLI commands.

mod compile;
mod run;
mod settings;

pub use self::{compile::*, run::*, settings::*};

#[cfg(feature = "serve")]
mod serve;
#[cfg(feature = "serve")]
pub use self::serve::*;

#[cfg(feature = "explore")]
mod explore;
#[cfg(feature = "explore")]
pub use self::explore::*;

#[cfg(feature = "wast")]
mod wast;
#[cfg(feature = "wast")]
pub use self::wast::*;

#[cfg(feature = "cache")]
mod config;
#[cfg(feature = "cache")]
pub use self::config::*;
