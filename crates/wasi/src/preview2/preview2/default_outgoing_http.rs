use crate::{
    wasi,
    wasi::types::{FutureIncomingResponse as Response, OutgoingRequest as Request, RequestOptions},
    WasiView,
};

#[async_trait::async_trait]
impl<T: WasiView> wasi::default_outgoing_http::Host for T {
    async fn handle(
        &mut self,
        _req: Request,
        _options: Option<RequestOptions>,
    ) -> wasmtime::Result<Response> {
        anyhow::bail!("not implemented")
    }
}
