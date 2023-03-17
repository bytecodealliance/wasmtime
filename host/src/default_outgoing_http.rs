use crate::{
    proxy::wasi,
    proxy::wasi::types::{
        FutureIncomingResponse as Response, OutgoingRequest as Request, RequestOptions,
    },
    WasiCtx,
};

#[async_trait::async_trait]
impl wasi::default_outgoing_http::Host for WasiCtx {
    async fn handle(
        &mut self,
        _req: Request,
        _options: Option<RequestOptions>,
    ) -> wasmtime::Result<Response> {
        anyhow::bail!("not implemented")
    }
}
