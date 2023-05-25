use crate::preview2::wasi::cli_base::environment;
use crate::preview2::wasi::cli_base::preopens;
use crate::preview2::wasi::filesystem::filesystem;
use crate::preview2::WasiView;

#[async_trait::async_trait]
impl<T: WasiView> environment::Host for T {
    async fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.ctx().env.clone())
    }
    async fn get_arguments(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.ctx().args.clone())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> preopens::Host for T {
    async fn get_stdio(&mut self) -> Result<preopens::StdioPreopens, anyhow::Error> {
        Ok(preopens::StdioPreopens {
            stdin: self.ctx().stdin,
            stdout: self.ctx().stdout,
            stderr: self.ctx().stderr,
        })
    }
    async fn get_directories(
        &mut self,
    ) -> Result<Vec<(filesystem::Descriptor, String)>, anyhow::Error> {
        Ok(self.ctx().preopens.clone())
    }
}
