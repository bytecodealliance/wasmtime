use crate::{command::wasi::stderr, WasiCtx};

#[async_trait::async_trait]
impl stderr::Host for WasiCtx {
    async fn print(&mut self, message: String) -> anyhow::Result<()> {
        eprint!("{}", message);
        Ok(())
    }
}
