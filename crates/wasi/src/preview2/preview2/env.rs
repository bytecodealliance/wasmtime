use crate::preview2::wasi::cli_base::environment;
use crate::preview2::wasi::cli_base::preopens;
use crate::preview2::wasi::cli_base::stderr;
use crate::preview2::wasi::cli_base::stdin;
use crate::preview2::wasi::cli_base::stdout;
use crate::preview2::wasi::filesystem::filesystem;
use crate::preview2::wasi::io::streams;
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
    async fn get_directories(
        &mut self,
    ) -> Result<Vec<(filesystem::Descriptor, String)>, anyhow::Error> {
        Ok(self.ctx().preopens.clone())
    }
}

#[async_trait::async_trait]
impl<T: WasiView> stdin::Host for T {
    async fn get_stdin(&mut self) -> Result<streams::InputStream, anyhow::Error> {
        Ok(self.ctx().stdin)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> stdout::Host for T {
    async fn get_stdout(&mut self) -> Result<streams::OutputStream, anyhow::Error> {
        Ok(self.ctx().stdout)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> stderr::Host for T {
    async fn get_stderr(&mut self) -> Result<streams::OutputStream, anyhow::Error> {
        Ok(self.ctx().stderr)
    }
}
