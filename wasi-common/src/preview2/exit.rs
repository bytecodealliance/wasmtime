use crate::wasi::exit;
use crate::{I32Exit, WasiCtx};

#[async_trait::async_trait]
impl exit::Host for WasiCtx {
    async fn exit(&mut self, status: Result<(), ()>) -> anyhow::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(anyhow::anyhow!(I32Exit(status)))
    }
}
