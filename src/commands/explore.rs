//! The module that implements the `wasmtime explore` command.

use anyhow::{Context, Result};
use clap::Parser;
use std::{borrow::Cow, path::PathBuf};
use wasmtime_cli_flags::CommonOptions;

/// Explore the compilation of a WebAssembly module to native code.
#[derive(Parser, PartialEq)]
pub struct ExploreCommand {
    #[command(flatten)]
    common: CommonOptions,

    /// The target triple; default is the host triple
    #[arg(long, value_name = "TARGET")]
    target: Option<String>,

    /// The path of the WebAssembly module to compile
    #[arg(required = true, value_name = "MODULE")]
    module: PathBuf,

    /// The path of the explorer output (derived from the MODULE name if none
    /// provided)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl ExploreCommand {
    /// Executes the command.
    pub fn execute(mut self) -> Result<()> {
        self.common.init_logging()?;

        let config = self.common.config(self.target.as_deref())?;

        let bytes =
            Cow::Owned(std::fs::read(&self.module).with_context(|| {
                format!("failed to read Wasm module: {}", self.module.display())
            })?);
        #[cfg(feature = "wat")]
        let bytes = wat::parse_bytes(&bytes).map_err(|mut e| {
            e.set_path(&self.module);
            e
        })?;

        let output = self
            .output
            .clone()
            .unwrap_or_else(|| self.module.with_extension("explore.html"));
        let output_file = std::fs::File::create(&output)
            .with_context(|| format!("failed to create file: {}", output.display()))?;
        let mut output_file = std::io::BufWriter::new(output_file);

        wasmtime_explorer::generate(&config, self.target.as_deref(), &bytes, &mut output_file)?;
        println!("Exploration written to {}", output.display());
        Ok(())
    }
}
