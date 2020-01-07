//! The module that implements the `wasmtime wast` command.

use crate::{init_file_per_thread_logger, pick_compilation_strategy, CommonOptions};
use anyhow::{bail, Context as _, Result};
use std::{fmt::Write, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};
use wasmtime::{Config, Engine, Store};
use wasmtime_environ::cache_init;
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
        let log_config = if self.common.debug {
            pretty_env_logger::init();
            None
        } else {
            let prefix = "wast.dbg.";
            init_file_per_thread_logger(prefix);
            Some(prefix)
        };

        let errors = cache_init(
            !self.common.disable_cache,
            self.common.config.as_ref(),
            log_config,
        );

        if !errors.is_empty() {
            let mut message = String::new();
            writeln!(message, "Cache initialization failed. Errors:")?;
            for e in errors {
                writeln!(message, "  -> {}", e)?;
            }
            bail!(message);
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
