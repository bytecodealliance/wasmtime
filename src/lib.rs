#![deny(
    // missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

mod ctx;
mod fdentry;
mod helpers;
mod hostcalls_impl;
mod sys;
#[macro_use]
mod macros;

pub mod host;
pub mod hostcalls;
pub mod memory;
pub mod wasm32;

pub use ctx::{WasiCtx, WasiCtxBuilder};
pub use sys::preopen_dir;

pub(crate) type Result<T> = std::result::Result<T, self::host::__wasi_errno_t>;
