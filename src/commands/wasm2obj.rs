//! The module that implements the `wasmtime wasm2obj` command.

use crate::obj::compile_to_obj;
use crate::{init_file_per_thread_logger, pick_compilation_strategy, CommonOptions};
use anyhow::{anyhow, Context as _, Result};
use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};
use structopt::{clap::AppSettings, StructOpt};
use target_lexicon::Triple;
use wasmtime_environ::CacheConfig;
#[cfg(feature = "lightbeam")]
use wasmtime_environ::Lightbeam;

/// The after help text for the `wasm2obj` command.
pub const WASM2OBJ_AFTER_HELP: &str = "The translation is dependent on the environment chosen.\n\
     The default is a dummy environment that produces placeholder values.";

fn parse_target(s: &str) -> Result<Triple> {
    Triple::from_str(&s).map_err(|e| anyhow!(e))
}

/// Translates a WebAssembly module to native object file
#[derive(StructOpt)]
#[structopt(
    name = "wasm2obj",
    version = env!("CARGO_PKG_VERSION"),
    setting = AppSettings::ColoredHelp,
    after_help = WASM2OBJ_AFTER_HELP,
)]
pub struct WasmToObjCommand {
    #[structopt(flatten)]
    common: CommonOptions,

    /// The path of the WebAssembly module to translate
    #[structopt(index = 1, value_name = "MODULE_PATH", parse(from_os_str))]
    module: PathBuf,

    /// The path of the output object file
    #[structopt(index = 2, value_name = "OUTPUT_PATH")]
    output: String,

    /// The target triple; default is the host triple
    #[structopt(long, value_name = "TARGET", parse(try_from_str = parse_target))]
    target: Option<Triple>,
}

impl WasmToObjCommand {
    /// Executes the command.
    pub fn execute(&self) -> Result<()> {
        self.handle_module()
    }

    fn handle_module(&self) -> Result<()> {
        if self.common.debug {
            pretty_env_logger::init();
        } else {
            let prefix = "wasm2obj.dbg.";
            init_file_per_thread_logger(prefix);
        }

        let cache_config = if self.common.disable_cache {
            CacheConfig::new_cache_disabled()
        } else {
            CacheConfig::from_file(self.common.config.as_deref())?
        };
        let strategy = pick_compilation_strategy(self.common.cranelift, self.common.lightbeam)?;

        let data = wat::parse_file(&self.module).context("failed to parse module")?;

        let obj = compile_to_obj(
            &data,
            self.target.as_ref(),
            strategy,
            self.common.enable_simd,
            self.common.optimize,
            self.common.debug_info,
            self.output.clone(),
            &cache_config,
        )?;

        // FIXME: Make the format a parameter.
        let file = File::create(Path::new(&self.output)).context("failed to create object file")?;
        obj.write(file).context("failed to write object file")?;

        Ok(())
    }
}
