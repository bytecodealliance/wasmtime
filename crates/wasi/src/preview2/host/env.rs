use crate::preview2::bindings::cli::environment;
use crate::preview2::WasiView;
use wasmtime::component::ResourceTable;

impl<T: WasiView> environment::Host for T {
    fn get_environment(&mut self, _: &mut ResourceTable) -> anyhow::Result<Vec<(String, String)>> {
        Ok(self.ctx().env.clone())
    }
    fn get_arguments(&mut self, _: &mut ResourceTable) -> anyhow::Result<Vec<String>> {
        Ok(self.ctx().args.clone())
    }
    fn initial_cwd(&mut self, _: &mut ResourceTable) -> anyhow::Result<Option<String>> {
        // FIXME: expose cwd in builder and save in ctx
        Ok(None)
    }
}
