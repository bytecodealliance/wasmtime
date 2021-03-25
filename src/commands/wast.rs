//! The module that implements the `wasmtime wast` command.

use crate::{init_file_per_thread_logger, CommonOptions};
use anyhow::{Context as _, Result};
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};
use wasmtime::{Engine, Store};
use wasmtime_wast::WastContext;

/// Runs a WebAssembly test script file
#[derive(StructOpt)]
#[structopt(
    name = "wast",
    version = env!("CARGO_PKG_VERSION"),
    setting = AppSettings::ColoredHelp,
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
        if !self.common.disable_logging {
            if self.common.log_to_files {
                let prefix = "wast.dbg.";
                init_file_per_thread_logger(prefix);
            } else {
                pretty_env_logger::init();
            }
        }

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
