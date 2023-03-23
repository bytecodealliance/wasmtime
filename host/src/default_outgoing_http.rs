use crate::{
    command, proxy,
    proxy::wasi::types::{FutureIncomingResponse, OutgoingRequest, RequestOptions},
    WasiCtx,
};

#[async_trait::async_trait]
impl command::wasi::default_outgoing_http::Host for WasiCtx {
    async fn handle(
        &mut self,
        _req: OutgoingRequest,
        _options: Option<command::wasi::types::RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse> {
        anyhow::bail!("not implemented")
    }
}

#[async_trait::async_trait]
impl proxy::wasi::default_outgoing_http::Host for WasiCtx {
    async fn handle(
        &mut self,
        _req: OutgoingRequest,
        _options: Option<RequestOptions>,
    ) -> wasmtime::Result<FutureIncomingResponse> {
        anyhow::bail!("not implemented")
    }
}
