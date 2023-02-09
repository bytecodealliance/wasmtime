use crate::{wasi_environment, WasiCtx};

#[async_trait::async_trait]
impl wasi_environment::WasiEnvironment for WasiCtx {
    async fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.env.clone())
    }
}
