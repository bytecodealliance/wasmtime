//! This crate contains the binaries of three WebAssembly modules:
//!
//! - [`WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER`]
//! - [`WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER`]
//! - [`WASI_SNAPSHOT_PREVIEW1_PROXY_ADAPTER`]
//!
//! These three modules bridge the wasip1 ABI to the wasip2 ABI of the component
//! model.
//!
//! They can be given to the [`wit_component::ComponentEncoder::adapter`] method
//! to translate a module from the historical WASM ABI to the canonical ABI.
//!
//! [`wit_component::ComponentEncoder::adapter`]: https://docs.rs/wit-component/latest/wit_component/struct.ComponentEncoder.html#method.adapter

/// The "reactor" adapter provides the default adaptation from preview1 to
/// preview2.
///
/// This adapter implements the [`wasi:cli/imports`] world.
///
/// [`wasi:cli/imports`]: https://github.com/WebAssembly/WASI/blob/01bb90d8b66cbc1d50349aaaab9ac5b143c9c98c/preview2/cli/imports.wit
pub const WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER: &[u8] =
    include_bytes!("../artefacts/wasi_snapshot_preview1.reactor.wasm");

/// The "command" adapter extends the ["reactor" adapter] and additionally
/// exports a `run` function entrypoint.
///
/// This adapter implements the [`wasi:cli/command`] world.
///
/// ["reactor" adapter]: WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER
/// [`wasi:cli/command`]: https://github.com/WebAssembly/WASI/blob/01bb90d8b66cbc1d50349aaaab9ac5b143c9c98c/preview2/cli/command.wit
pub const WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER: &[u8] =
    include_bytes!("../artefacts/wasi_snapshot_preview1.command.wasm");

/// The "proxy" adapter provides implements a HTTP proxy which is more
/// restricted than the ["reactor" adapter] adapter, as it lacks filesystem,
/// socket, environment, exit, and terminal support, but includes HTTP handlers
/// for incoming and outgoing requests.
///
/// This adapter implements the [`wasi:http/proxy`] world.
///
/// ["reactor" adapter]: WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER
/// [`wasi:http/proxy`]: https://github.com/WebAssembly/WASI/blob/01bb90d8b66cbc1d50349aaaab9ac5b143c9c98c/preview2/http/proxy.wit
pub const WASI_SNAPSHOT_PREVIEW1_PROXY_ADAPTER: &[u8] =
    include_bytes!("../artefacts/wasi_snapshot_preview1.proxy.wasm");
