use super::{mk_delete, mk_get};
use crate::p3::WasiTlsCtxView;
use crate::p3::bindings::tls::types::{Error, Host, HostError};
use wasmtime::component::Resource;

mk_get!(Error, get_error, "error");
mk_delete!(Error, delete_error, "error");

impl Host for WasiTlsCtxView<'_> {}

impl HostError for WasiTlsCtxView<'_> {
    fn to_debug_string(&mut self, err: Resource<Error>) -> wasmtime::Result<String> {
        let err = get_error(self.table, &err)?;
        Ok(err.clone())
    }

    fn drop(&mut self, err: Resource<Error>) -> wasmtime::Result<()> {
        delete_error(&mut self.table, err)?;
        Ok(())
    }
}
