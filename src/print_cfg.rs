//! The `print-cfg` sub-command.
//!
//! Read a series of Cretonne IR files and print their control flow graphs
//! in graphviz format.

use cretonne_codegen::cfg_printer::CFGPrinter;
use cretonne_reader::parse_functions;
use utils::read_to_string;
use CommandResult;

pub fn run(files: &[String]) -> CommandResult {
    for (i, f) in files.into_iter().enumerate() {
        if i != 0 {
            println!();
        }
        print_cfg(f)?
    }
    Ok(())
}

fn print_cfg(filename: &str) -> CommandResult {
    let buffer = read_to_string(filename).map_err(|e| format!("{}: {}", filename, e))?;
    let items = parse_functions(&buffer).map_err(|e| format!("{}: {}", filename, e))?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!();
        }
        print!("{}", CFGPrinter::new(&func));
    }

    Ok(())
}
