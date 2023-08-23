use crate::bindings::http::types::{IncomingRequest, ResponseOutparam};
use crate::WasiHttpView;

#[async_trait::async_trait]
impl<T: WasiHttpView> crate::bindings::http::incoming_handler::Host for T {
    async fn handle(
        &mut self,
        _request: IncomingRequest,
        _response_out: ResponseOutparam,
    ) -> wasmtime::Result<()> {
        anyhow::bail!("unimplemented: [incoming_handler] handle")
    }
}
