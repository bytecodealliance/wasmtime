//! This crate contains the binaries of three WebAssembly modules:
//!
//! - [`WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER`]
//! - [`WASI_SNAPSHOT_PREVIEW1_PROXY_ADAPTER`]
//! - [`WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER`]
//!
//! These three modules bridge the wasip1 ABI to the wasip2 ABI of the component
//! model.
//!
//! They can be given to the [`wit_component::ComponentEncoder::adapter`] method
//! to translate a module from the historical WASM ABI to the canonical ABI.
//!
//! [`wit_component::ComponentEncoder::adapter`]: https://docs.rs/wit-component/latest/wit_component/struct.ComponentEncoder.html#method.adapter

/// The "command" adapter extends the ["reactor" adapter] and additionally
/// exports a `run` function entrypoint.
///
/// This adapter implements the [`wasi:cli/command`] world.
///
/// ["reactor" adapter]: WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER
/// [`wasi:cli/command`]: https://github.com/WebAssembly/wasi-cli/blob/6ae82617096e83e6606047736e84ac397b788631/wit/command.wit
pub const WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER: &[u8] =
    include_bytes!("../artefacts/wasi_snapshot_preview1.command.wasm");

/// The "proxy" adapter provides implements a HTTP proxy.
pub const WASI_SNAPSHOT_PREVIEW1_PROXY_ADAPTER: &[u8] =
    include_bytes!("../artefacts/wasi_snapshot_preview1.proxy.wasm");

/// The "reactor" adapter provides the default adaptation from preview1 to
/// preview2.
///
/// This adapter implements the [`wasi:cli/imports`] world.
///
/// [`wasi:cli/imports`]: https://github.com/WebAssembly/wasi-cli/blob/6ae82617096e83e6606047736e84ac397b788631/wit/imports.wit
pub const WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER: &[u8] =
    include_bytes!("../artefacts/wasi_snapshot_preview1.reactor.wasm");
