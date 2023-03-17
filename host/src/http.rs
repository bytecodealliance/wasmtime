use crate::{
    wasi,
    wasi::types::{IncomingRequest as Request, ResponseOutparam as Response},
    WasiCtx,
};

#[async_trait::async_trait]
impl wasi::http::Host for WasiCtx {
    async fn handle(&mut self, _req: Request, _resp: Response) -> anyhow::Result<()> {
        anyhow::bail!("not implemented")
    }
}
