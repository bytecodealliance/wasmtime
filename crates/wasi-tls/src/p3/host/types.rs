use crate::p3::WasiTlsCtxView;
use crate::p3::bindings::tls::types::{Error, Host, HostError};
use wasmtime::component::Resource;

impl Host for WasiTlsCtxView<'_> {}

impl HostError for WasiTlsCtxView<'_> {
    fn to_debug_string(&mut self, err: Resource<Error>) -> wasmtime::Result<String> {
        let err = self.table.get(&err)?;
        Ok(err.clone())
    }

    fn drop(&mut self, err: Resource<Error>) -> wasmtime::Result<()> {
        self.table.delete(err)?;
        Ok(())
    }
}
