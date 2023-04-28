use crate::wasi;
use crate::WasiCtx;

#[async_trait::async_trait]
impl wasi::environment::Host for WasiCtx {
    async fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.env.clone())
    }
    async fn get_arguments(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.args.clone())
    }
}

#[async_trait::async_trait]
impl wasi::preopens::Host for WasiCtx {
    async fn get_stdio(&mut self) -> Result<wasi::preopens::StdioPreopens, anyhow::Error> {
        Ok(wasi::preopens::StdioPreopens {
            stdin: self.stdin,
            stdout: self.stdout,
            stderr: self.stderr,
        })
    }
    async fn get_directories(
        &mut self,
    ) -> Result<Vec<(wasi::filesystem::Descriptor, String)>, anyhow::Error> {
        Ok(self.preopens.clone())
    }
}
