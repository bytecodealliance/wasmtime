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

#[cfg(feature = "sync")]
pub mod sync {
    use crate::bindings::http::incoming_handler::Host as AsyncHost;
    use crate::bindings::sync::http::types::{IncomingRequest, ResponseOutparam};
    use crate::WasiHttpView;
    use wasmtime_wasi::preview2::in_tokio;

    impl<T: WasiHttpView> crate::bindings::sync::http::incoming_handler::Host for T {
        fn handle(
            &mut self,
            request: IncomingRequest,
            response_out: ResponseOutparam,
        ) -> wasmtime::Result<()> {
            in_tokio(async { AsyncHost::handle(self, request, response_out).await })
        }
    }
}
