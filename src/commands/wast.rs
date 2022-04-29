//! The module that implements the `wasmtime wast` command.

use crate::CommonOptions;
use anyhow::{Context as _, Result};
use clap::Parser;
use std::path::PathBuf;
use wasmtime::{Engine, Store};
use wasmtime_wast::WastContext;

lazy_static::lazy_static! {
    static ref AFTER_HELP: String = {
        crate::FLAG_EXPLANATIONS.to_string()
    };
}

/// Runs a WebAssembly test script file
#[derive(Parser)]
#[clap(
    name = "wast",
    version,
    after_help = AFTER_HELP.as_str(),
)]
pub struct WastCommand {
    #[clap(flatten)]
    common: CommonOptions,

    /// The path of the WebAssembly test script to run
    #[clap(required = true, value_name = "SCRIPT_FILE", parse(from_os_str))]
    scripts: Vec<PathBuf>,
}

impl WastCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        self.common.init_logging();

        let config = self.common.config(None)?;
        let store = Store::new(&Engine::new(&config)?, ());
        let mut wast_context = WastContext::new(store);

        wast_context
            .register_spectest()
            .expect("error instantiating \"spectest\"");

        for script in self.scripts.iter() {
            wast_context
                .run_file(script)
                .with_context(|| format!("failed to run script file '{}'", script.display()))?
        }

        Ok(())
    }
}
