//! The module that implements the `wasmtime wast` command.

use anyhow::{Context as _, Result};
use clap::Parser;
use std::path::PathBuf;
use wasmtime::{Engine, Store};
use wasmtime_cli_flags::CommonOptions;
use wasmtime_wast::{SpectestConfig, WastContext};

/// Runs a WebAssembly test script file
#[derive(Parser)]
pub struct WastCommand {
    #[command(flatten)]
    common: CommonOptions,

    /// The path of the WebAssembly test script to run
    #[arg(required = true, value_name = "SCRIPT_FILE")]
    scripts: Vec<PathBuf>,
}

impl WastCommand {
    /// Executes the command.
    pub fn execute(mut self) -> Result<()> {
        self.common.init_logging()?;

        let mut config = self.common.config(None)?;
        config.async_support(true);
        let mut store = Store::new(&Engine::new(&config)?, ());
        if let Some(fuel) = self.common.wasm.fuel {
            store.set_fuel(fuel)?;
        }
        if let Some(true) = self.common.wasm.epoch_interruption {
            store.epoch_deadline_trap();
            store.set_epoch_deadline(1);
        }
        let mut wast_context = WastContext::new(store, wasmtime_wast::Async::Yes);

        wast_context
            .register_spectest(&SpectestConfig {
                use_shared_memory: true,
                suppress_prints: false,
            })
            .expect("error instantiating \"spectest\"");

        for script in self.scripts.iter() {
            wast_context
                .run_file(script)
                .with_context(|| format!("failed to run script file '{}'", script.display()))?
        }

        Ok(())
    }
}
