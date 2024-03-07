use crate::bindings::cli::environment;
use crate::WasiView;

impl<T: WasiView> environment::Host for T {
    fn get_environment(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.ctx().env.clone())
    }
    fn get_arguments(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.ctx().args.clone())
    }
    fn initial_cwd(&mut self) -> anyhow::Result<Option<String>> {
        // FIXME: expose cwd in builder and save in ctx
        Ok(None)
    }
}
