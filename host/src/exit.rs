use crate::command::wasi::exit;
use crate::WasiCtx;

#[async_trait::async_trait]
impl exit::Host for WasiCtx {
    async fn exit(&mut self, status: Result<(), ()>) -> anyhow::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(anyhow::anyhow!(wasi_common::I32Exit(status)))
    }
}
