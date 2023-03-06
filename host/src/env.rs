use crate::wasi;
use crate::WasiCtx;

#[async_trait::async_trait]
impl wasi::environment::Host for WasiCtx {
    async fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.env.clone())
    }
}

#[async_trait::async_trait]
impl wasi::environment_preopens::Host for WasiCtx {
    async fn preopens(
        &mut self,
    ) -> Result<Vec<(wasi::filesystem::Descriptor, String)>, anyhow::Error> {
        // Create new handles to the preopens.
        let mut results = Vec::new();
        for (handle, name) in &self.preopens {
            let desc = self.table.push(Box::new(handle.dup()))?;
            results.push((desc, name.clone()));
        }
        Ok(results)
    }
}
