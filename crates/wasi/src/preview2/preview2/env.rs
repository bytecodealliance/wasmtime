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
    async fn get_directories(
        &mut self,
    ) -> Result<Vec<(wasi::filesystem::Descriptor, String)>, anyhow::Error> {
        Ok(self.ctx().preopens.clone())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> wasi::stdin::Host for T {
    async fn get_stdin(&mut self) -> Result<wasi::streams::InputStream, anyhow::Error> {
        Ok(self.ctx().stdin)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> wasi::stdout::Host for T {
    async fn get_stdout(&mut self) -> Result<wasi::streams::OutputStream, anyhow::Error> {
        Ok(self.ctx().stdout)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> wasi::stderr::Host for T {
    async fn get_stderr(&mut self) -> Result<wasi::streams::OutputStream, anyhow::Error> {
        Ok(self.ctx().stderr)
    }
}
