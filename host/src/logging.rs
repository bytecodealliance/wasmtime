#![allow(unused_variables)]

use crate::{wasi_logging, WasiCtx};

impl wasi_logging::WasiLogging for WasiCtx {
    fn log(
        &mut self,
        level: wasi_logging::Level,
        context: String,
        message: String,
    ) -> anyhow::Result<()> {
        println!("{:?} {}: {}", level, context, message);
        Ok(())
    }
}
