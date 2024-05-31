//! The module for the Wasmtime CLI commands.
sudo apt
#[cfg(feature = "run")]
mod run;
#[cfg(feature = "run")]
pub use self::run::*;

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

#[cfg(feature = "compile")]
mod compile;
#[cfg(feature = "compile")]
pub use self::compile::*;

#[cfg(feature = "cranelift")]
mod settings;
#[cfg(feature = "cranelift")]
pub use self::settings::*;
