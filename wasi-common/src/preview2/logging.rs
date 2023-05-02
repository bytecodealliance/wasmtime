use crate::wasi::console;
use crate::WasiView;

#[async_trait::async_trait]
impl<T: WasiView> console::Host for T {
    async fn log(
        &mut self,
        level: console::Level,
        context: String,
        message: String,
    ) -> anyhow::Result<()> {
        println!("{:?} {}: {}", level, context, message);
        Ok(())
    }
}
