use crate::preview2::{wasi::cli_base::exit, I32Exit, WasiView};

#[async_trait::async_trait]
impl<T: WasiView> exit::Host for T {
    async fn exit(&mut self, status: Result<(), ()>) -> anyhow::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(anyhow::anyhow!(I32Exit(status)))
    }
}
