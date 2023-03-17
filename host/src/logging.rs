use crate::proxy::wasi::console;
use crate::WasiCtx;

#[async_trait::async_trait]
impl console::Host for WasiCtx {
    async fn log(
        &mut self,
        level: console::Level,
        context: String,
        message: String,
    ) -> anyhow::Result<()> {
        println!("{:?} {}: {}", level, context, message);
        Ok(())
    }
}
