//! The module that implements the `wasmtime wast` command.

use crate::CommonOptions;
use anyhow::{Context as _, Result};
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};
use wasmtime::{Engine, Store};
use wasmtime_wast::WastContext;

lazy_static::lazy_static! {
    static ref AFTER_HELP: String = {
        crate::FLAG_EXPLANATIONS.to_string()
    };
}

/// Runs a WebAssembly test script file
#[derive(StructOpt)]
#[structopt(
    name = "wast",
    version = env!("CARGO_PKG_VERSION"),
    setting = AppSettings::ColoredHelp,
    after_help = AFTER_HELP.as_str(),
)]
pub struct WastCommand {
    #[structopt(flatten)]
    common: CommonOptions,

    /// The path of the WebAssembly test script to run
    #[structopt(required = true, value_name = "SCRIPT_FILE", parse(from_os_str))]
    scripts: Vec<PathBuf>,
}

impl WastCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        self.common.init_logging();

        let config = self.common.config(None)?;
        let store = Store::new(&Engine::new(&config)?);
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
