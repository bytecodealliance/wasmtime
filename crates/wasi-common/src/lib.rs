#![deny(
    // missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features,
    clippy::use_self
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
mod error;
mod fdentry;
pub mod fs;
mod helpers;
mod host;
mod hostcalls_impl;
mod memory;
pub mod old;
mod sandboxed_tty_writer;
mod sys;
mod virtfs;
pub use virtfs::{FileContents, VirtualDirEntry};
pub mod wasi;
pub mod wasi32;

pub mod hostcalls {
    wig::define_hostcalls!("snapshot" "wasi_snapshot_preview1");
}

pub use ctx::{WasiCtx, WasiCtxBuilder};
pub use sys::preopen_dir;

pub use error::Error;
pub(crate) use error::Result;
