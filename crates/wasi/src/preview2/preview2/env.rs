use crate::preview2::bindings::cli_base::{environment, stderr, stdin, stdout};
use crate::preview2::bindings::io::streams;
use crate::preview2::WasiView;

impl<T: WasiView> environment::Host for T {
    fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.ctx().env.clone())
    }
    fn get_arguments(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.ctx().args.clone())
    }
}

impl<T: WasiView> stdin::Host for T {
    fn get_stdin(&mut self) -> Result<streams::InputStream, anyhow::Error> {
        Ok(self.ctx().stdin)
    }
}

impl<T: WasiView> stdout::Host for T {
    fn get_stdout(&mut self) -> Result<streams::OutputStream, anyhow::Error> {
        Ok(self.ctx().stdout)
    }
}

impl<T: WasiView> stderr::Host for T {
    fn get_stderr(&mut self) -> Result<streams::OutputStream, anyhow::Error> {
        Ok(self.ctx().stderr)
    }
}
