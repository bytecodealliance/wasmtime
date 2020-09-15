//! The `print-cfg` sub-command.
//!
//! Read a series of Cranelift IR files and print their control flow graphs
//! in graphviz format.

use crate::utils::read_to_string;
use anyhow::Result;
use cranelift_codegen::cfg_printer::CFGPrinter;
use cranelift_reader::parse_functions;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// Prints out cfg in GraphViz Dot format
#[derive(StructOpt)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    #[structopt(required(true), parse(from_os_str))]
    files: Vec<PathBuf>,

    /// Enable debug output on stderr/stdout
    #[structopt(short = "d")]
    debug: bool,
}

pub fn run(options: &Options) -> Result<()> {
    crate::handle_debug_flag(options.debug);
    for (i, f) in options.files.iter().enumerate() {
        if i != 0 {
            println!();
        }
        print_cfg(f)?
    }
    Ok(())
}

fn print_cfg(path: &Path) -> Result<()> {
    let buffer = read_to_string(path)?;
    let items = parse_functions(&buffer)?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!();
        }
        print!("{}", CFGPrinter::new(&func));
    }

    Ok(())
}
