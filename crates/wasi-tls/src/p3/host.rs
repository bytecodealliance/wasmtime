//! p3 host implementation scaffolding for `wasi:tls`.
//!
//! This module will contain resource/state wiring for `wasi:tls/client` and
//! `wasi:tls/types` once the stream-transform connector behavior is implemented.

use crate::p3::{WasiTls, WasiTlsCtxView, bindings};
use wasmtime::component::{Access, Accessor, FutureReader, Resource, StreamReader};

/// Host-side state stored for `wasi:tls/client` `connector` resources.
pub struct Connector;

/// Host-side state stored for `wasi:tls/types` `error` resources.
pub struct Error(wasmtime::Error);

impl<'a> bindings::tls::client::Host for WasiTlsCtxView<'a> {}
impl<'a> bindings::tls::types::Host for WasiTlsCtxView<'a> {}

impl<'a> bindings::tls::types::HostError for WasiTlsCtxView<'a> {
    fn to_debug_string(
        &mut self,
        this: Resource<bindings::tls::types::Error>,
    ) -> wasmtime::Result<String> {
        Ok(self.table.get(&this)?.0.to_string())
    }

    fn drop(&mut self, rep: Resource<bindings::tls::types::Error>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl<'a> bindings::tls::client::HostConnector for WasiTlsCtxView<'a> {
    fn new(&mut self) -> wasmtime::Result<Resource<bindings::tls::client::Connector>> {
        Ok(self.table.push(Connector)?)
    }

    fn drop(&mut self, rep: Resource<bindings::tls::client::Connector>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl bindings::tls::client::HostConnectorWithStore for WasiTls {
    fn send<T: 'static>(
        _store: Access<'_, T, Self>,
        _this: Resource<bindings::tls::client::Connector>,
        _cleartext: StreamReader<u8>,
    ) -> wasmtime::Result<(
        StreamReader<u8>,
        FutureReader<Result<(), Resource<bindings::tls::types::Error>>>,
    )> {
        todo!()
    }

    fn receive<T: 'static>(
        _store: Access<'_, T, Self>,
        _this: Resource<bindings::tls::client::Connector>,
        _ciphertext: StreamReader<u8>,
    ) -> wasmtime::Result<(
        StreamReader<u8>,
        FutureReader<Result<(), Resource<bindings::tls::types::Error>>>,
    )> {
        todo!()
    }

    async fn connect<T: Send>(
        _accessor: &Accessor<T, Self>,
        _this: Resource<bindings::tls::client::Connector>,
        _server_name: String,
    ) -> wasmtime::Result<Result<(), Resource<bindings::tls::types::Error>>> {
        todo!()
    }
}
