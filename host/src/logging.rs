use crate::{wasi_logging, WasiCtx};

#[async_trait::async_trait]
impl wasi_logging::WasiLogging for WasiCtx {
    async fn log(
        &mut self,
        level: wasi_logging::Level,
        context: String,
        message: String,
    ) -> anyhow::Result<()> {
        println!("{:?} {}: {}", level, context, message);
        Ok(())
    }
}
