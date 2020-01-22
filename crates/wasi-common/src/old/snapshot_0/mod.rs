mod ctx;
mod error;
mod fdentry;
mod helpers;
mod host;
mod hostcalls_impl;
mod memory;
mod sys;
pub mod wasi;
pub mod wasi32;

pub mod hostcalls {
    wig::define_hostcalls!("old/snapshot_0" "wasi_unstable");
}

pub use ctx::{WasiCtx, WasiCtxBuilder};

pub type Error = error::Error;
pub type Result<T> = std::result::Result<T, Error>;
