//! The `cat` sub-command.
//!
//! Read a sequence of Cranelift IR files and print them again to stdout. This has the effect of
//! normalizing formatting and removing comments.

use crate::utils::read_to_string;
use anyhow::{Context, Result};
use clap::Parser;
use cranelift_reader::parse_functions;
use std::path::{Path, PathBuf};

/// Outputs .clif file
#[derive(Parser)]
pub struct Options {
    /// Specify input file(s) to be used. Use '-' for stdin.
    #[clap(required = true)]
    files: Vec<PathBuf>,
}

pub fn run(options: &Options) -> Result<()> {
    for (i, f) in options.files.iter().enumerate() {
        if i != 0 {
            println!();
        }
        cat_one(f)?
    }
    Ok(())
}

fn cat_one(path: &Path) -> Result<()> {
    let buffer = read_to_string(path)?;
    let items =
        parse_functions(&buffer).with_context(|| format!("failed to parse {}", path.display()))?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!();
        }
        print!("{}", func);
    }

    Ok(())
}
