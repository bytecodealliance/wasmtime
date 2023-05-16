use crate::preview2::{wasi, WasiView};

#[async_trait::async_trait]
impl<T: WasiView> wasi::environment::Host for T {
    async fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.ctx().env.clone())
    }
    async fn get_arguments(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.ctx().args.clone())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> wasi::preopens::Host for T {
    async fn get_stdio(&mut self) -> Result<wasi::preopens::StdioPreopens, anyhow::Error> {
        Ok(wasi::preopens::StdioPreopens {
            stdin: self.ctx().stdin,
            stdout: self.ctx().stdout,
            stderr: self.ctx().stderr,
        })
    }
    async fn get_directories(
        &mut self,
    ) -> Result<Vec<(wasi::filesystem::Descriptor, String)>, anyhow::Error> {
        Ok(self.ctx().preopens.clone())
    }
}
