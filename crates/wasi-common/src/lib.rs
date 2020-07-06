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
mod entry;
mod fdpool;
pub mod fs;
mod handle;
pub mod old;
mod path;
mod poll;
mod sandboxed_tty_writer;
pub mod snapshots;
mod sys;
pub mod virtfs;
pub mod wasi;

pub use ctx::{WasiCtx, WasiCtxBuilder, WasiCtxBuilderError};
pub use handle::{Handle, HandleRights};
pub use sys::osdir::OsDir;
pub use sys::osfile::OsFile;
pub use sys::osother::OsOther;
pub use sys::preopen_dir;
pub use virtfs::{FileContents, VirtualDirEntry};
