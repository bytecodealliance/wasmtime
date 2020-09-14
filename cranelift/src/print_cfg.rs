//! The `print-cfg` sub-command.
//!
//! Read a series of Cranelift IR files and print their control flow graphs
//! in graphviz format.

use crate::utils::read_to_string;
use anyhow::Result;
use cranelift_codegen::cfg_printer::CFGPrinter;
use cranelift_reader::parse_functions;

pub fn run(files: &[String]) -> Result<()> {
    for (i, f) in files.iter().enumerate() {
        if i != 0 {
            println!();
        }
        print_cfg(f)?
    }
    Ok(())
}

fn print_cfg(filename: &str) -> Result<()> {
    let buffer = read_to_string(filename)?;
    let items = parse_functions(&buffer)?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!();
        }
        print!("{}", CFGPrinter::new(&func));
    }

    Ok(())
}
