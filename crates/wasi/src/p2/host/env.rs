use crate::cli::WasiCliCtxView;
use crate::p2::bindings::cli::environment;

impl environment::Host for WasiCliCtxView<'_> {
    fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.ctx.environment.clone())
    }
    fn get_arguments(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.ctx.arguments.clone())
    }
    fn initial_cwd(&mut self) -> anyhow::Result<Option<String>> {
        Ok(self.ctx.initial_cwd.clone())
    }
}
