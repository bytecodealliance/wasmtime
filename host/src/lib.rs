mod clocks;
mod default_outgoing_http;
mod env;
mod exit;
mod filesystem;
mod http_types;
mod io;
mod ip_name_lookup;
mod logging;
mod network;
mod poll;
mod random;
mod tcp;
mod udp;
pub use wasi_common::{table::Table, WasiCtx};

type HostResult<T, E> = anyhow::Result<Result<T, E>>;

pub mod command;
pub mod proxy;
