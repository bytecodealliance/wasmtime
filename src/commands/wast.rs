//! The module that implements the `wasmtime wast` command.

use crate::{init_file_per_thread_logger, pick_compilation_strategy, CommonOptions};
use anyhow::{Context as _, Result};
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};
use wasmtime::{Config, Engine, Store};
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
    pub fn execute(&self) -> Result<()> {
        if self.common.debug {
            pretty_env_logger::init();
        } else {
            let prefix = "wast.dbg.";
            init_file_per_thread_logger(prefix);
        }

        let mut config = Config::new();
        config
            .cranelift_debug_verifier(cfg!(debug_assertions))
            .debug_info(self.common.debug_info)
            .wasm_simd(self.common.enable_simd)
            .strategy(pick_compilation_strategy(
                self.common.cranelift,
                self.common.lightbeam,
            )?)?;
        self.common.configure_cache(&mut config)?;

        if self.common.optimize {
            config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        }

        let store = Store::new(&Engine::new(&config));
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
