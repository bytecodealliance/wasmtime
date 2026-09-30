use crate::I32Exit;
use crate::cli::WasiCliCtxView;
use crate::p2::bindings::cli::exit;

impl exit::Host for WasiCliCtxView<'_> {
    fn exit(&mut self, status: Result<(), ()>) -> wasmtime::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(wasmtime::format_err!(I32Exit(status)))
    }

    fn exit_with_code(&mut self, status_code: u8) -> wasmtime::Result<()> {
        Err(wasmtime::format_err!(I32Exit(status_code.into())))
    }
}

mod named {
    use crate::cli::WasiCliNamedView;
    use crate::p2::bindings::named_imports::wasi::cli::exit;
    use crate::{NamedId, WasiCtxNamedView};

    impl<T> exit::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn exit(&mut self, id: NamedId, status: Result<(), ()>) -> wasmtime::Result<()> {
            super::exit::Host::exit(&mut self.0.cli(id), status)
        }

        fn exit_with_code(&mut self, id: NamedId, status_code: u8) -> wasmtime::Result<()> {
            super::exit::Host::exit_with_code(&mut self.0.cli(id), status_code)
        }
    }
}
