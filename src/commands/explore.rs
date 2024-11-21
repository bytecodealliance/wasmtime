//! The module that implements the `wasmtime explore` command.

use anyhow::{Context, Result};
use clap::Parser;
use std::{borrow::Cow, path::PathBuf};
use tempfile::tempdir;
use wasmtime::Strategy;
use wasmtime_cli_flags::CommonOptions;

/// Explore the compilation of a WebAssembly module to native code.
#[derive(Parser)]
pub struct ExploreCommand {
    #[command(flatten)]
    common: CommonOptions,

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

        let mut config = self.common.config(None)?;

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

        let clif_dir = if let Some(Strategy::Cranelift) | None = self.common.codegen.compiler {
            let clif_dir = tempdir()?;
            config.emit_clif(clif_dir.path());
            config.disable_cache(); // cache does not emit clif
            Some(clif_dir)
        } else {
            None
        };

        wasmtime_explorer::generate(
            &config,
            self.common.target.as_deref(),
            clif_dir.as_ref().map(|tmp_dir| tmp_dir.path()),
            &bytes,
            &mut output_file,
        )?;

        println!("Exploration written to {}", output.display());
        Ok(())
    }
}
