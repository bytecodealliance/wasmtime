use crate::{bindings::cli::exit, I32Exit, WasiView};

impl exit::Host for dyn WasiView + '_ {
    fn exit(&mut self, status: Result<(), ()>) -> anyhow::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(anyhow::anyhow!(I32Exit(status)))
    }
}
