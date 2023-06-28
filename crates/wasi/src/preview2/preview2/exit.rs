use crate::preview2::{bindings::cli_base::exit, I32Exit, WasiView};

impl<T: WasiView> exit::Host for T {
    fn exit(&mut self, status: Result<(), ()>) -> anyhow::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(anyhow::anyhow!(I32Exit(status)))
    }
}
