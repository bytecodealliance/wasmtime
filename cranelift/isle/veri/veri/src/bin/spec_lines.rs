use std::sync::Arc;

use anyhow::{Result, bail, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle::{error::Errors, files::Files, lexer, parser};

#[derive(Parser)]
struct Opts {
    /// Name of the ISLE compilation.
    #[arg(long, required = true)]
    name: String,

    /// Path to codegen crate directory.
    #[arg(long, required = true)]
    codegen_crate_dir: std::path::PathBuf,

    /// Working directory.
    #[arg(long, required = true)]
    work_dir: std::path::PathBuf,
}

impl Opts {
    fn isle_input_files(&self) -> Result<Vec<std::path::PathBuf>> {
        // Generate ISLE files.
        let gen_dir = &self.work_dir;
        generate_isle(gen_dir)?;

        // Lookup ISLE compilations.
        let compilations = get_isle_compilations(&self.codegen_crate_dir, gen_dir);

        // Return inputs from the matching compilation, if any.
        Ok(compilations
            .lookup(&self.name)
            .ok_or(format_err!("unknown ISLE compilation: {}", self.name))?
            .paths()?)
    }
}

fn main() -> Result<()> {
    env_logger::builder()
        .format_level(false)
        .format_timestamp(None)
        .format_target(false)
        .init();
    let opts = Opts::parse();

    // Read ISLE inputs.
    let inputs = opts.isle_input_files()?;

    let files = match Files::from_paths(inputs, &[]) {
        Ok(files) => files,
        Err((path, err)) => {
            bail!(Errors::from_io(
                err,
                format!("cannot read file {}", path.display()),
            ))
        }
    };

    let files = Arc::new(files);

    let mut defs = Vec::new();
    for (file, src) in files.file_texts.iter().enumerate() {
        let lexer = match lexer::Lexer::new(file, src) {
            Ok(lexer) => lexer,
            Err(err) => bail!(Errors::new(vec![err], files)),
        };

        match parser::parse(lexer) {
            Ok(mut ds) => defs.append(&mut ds),
            Err(err) => bail!(Errors::new(vec![err], files)),
        }
    }

    Ok(())
}
