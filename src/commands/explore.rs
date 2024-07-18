//! The module that implements the `wasmtime explore` command.

use anyhow::{Context, Result};
use clap::Parser;
use std::{
    borrow::Cow,
    fs::{create_dir, remove_dir_all},
    io,
    path::PathBuf,
};
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

        let mut config = self.common.config(self.target.as_deref(), None)?;

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

        let clif_dir = output
            .parent()
            .map::<Result<PathBuf>, _>(|output_dir| {
                let clif_dir = output_dir.join("clif");
                if let Err(err) = create_dir(&clif_dir) {
                    match err.kind() {
                        io::ErrorKind::AlreadyExists => {}
                        _ => return Err(err.into()),
                    }
                }
                config.emit_clif(&clif_dir);
                Ok(clif_dir)
            })
            .transpose()?;

        wasmtime_explorer::generate(
            &config,
            self.target.as_deref(),
            clif_dir.as_deref(),
            &bytes,
            &mut output_file,
        )?;

        if let Some(clif_dir) = clif_dir {
            remove_dir_all(&clif_dir)?;
        }

        println!("Exploration written to {}", output.display());
        Ok(())
    }
}
