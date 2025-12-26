use super::mk_delete;
use crate::p3::WasiTlsCtxView;
use crate::p3::bindings::tls::types::{Certificate, Host, HostCertificate};
use wasmtime::component::Resource;

mk_delete!(Certificate, delete_certificate, "certificate");

impl Host for WasiTlsCtxView<'_> {}

impl HostCertificate for WasiTlsCtxView<'_> {
    fn drop(&mut self, cert: Resource<Certificate>) -> wasmtime::Result<()> {
        delete_certificate(&mut self.table, cert)?;
        Ok(())
    }
}
