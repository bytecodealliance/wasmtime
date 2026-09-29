use crate::cli::WasiCliCtxView;
use crate::p2::bindings::cli::environment;

impl environment::Host for WasiCliCtxView<'_> {
    fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        Ok(self.ctx.environment.clone())
    }
    fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(self.ctx.arguments.clone())
    }
    fn initial_cwd(&mut self) -> wasmtime::Result<Option<String>> {
        Ok(self.ctx.initial_cwd.clone())
    }
}

mod named {
    use crate::cli::WasiCliNamedView;
    use crate::p2::bindings::named_imports::wasi::cli::environment;
    use crate::{NamedId, WasiCtxNamedView};

    impl<T> environment::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_environment(&mut self, id: NamedId) -> wasmtime::Result<Vec<(String, String)>> {
            super::environment::Host::get_environment(&mut self.0.cli(id))
        }

        fn get_arguments(&mut self, id: NamedId) -> wasmtime::Result<Vec<String>> {
            super::environment::Host::get_arguments(&mut self.0.cli(id))
        }

        fn initial_cwd(&mut self, id: NamedId) -> wasmtime::Result<Option<String>> {
            super::environment::Host::initial_cwd(&mut self.0.cli(id))
        }
    }
}
