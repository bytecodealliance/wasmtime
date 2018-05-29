//! The `cat` sub-command.
//!
//! Read a sequence of Cretonne IR files and print them again to stdout. This has the effect of
//! normalizing formatting and removing comments.

use cretonne_reader::parse_functions;
use utils::read_to_string;
use CommandResult;

pub fn run(files: &[String]) -> CommandResult {
    for (i, f) in files.into_iter().enumerate() {
        if i != 0 {
            println!();
        }
        cat_one(f)?
    }
    Ok(())
}

fn cat_one(filename: &str) -> CommandResult {
    let buffer = read_to_string(&filename).map_err(|e| format!("{}: {}", filename, e))?;
    let items = parse_functions(&buffer).map_err(|e| format!("{}: {}", filename, e))?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!();
        }
        print!("{}", func);
    }

    Ok(())
}
