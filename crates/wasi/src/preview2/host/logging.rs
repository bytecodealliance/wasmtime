use crate::preview2::{bindings::logging::logging, WasiView};

impl<T: WasiView> logging::Host for T {
    fn log(
        &mut self,
        level: logging::Level,
        context: String,
        message: String,
    ) -> anyhow::Result<()> {
        eprintln!("{:?}: ({}) {}", level, context, message);
        Ok(())
    }
}
