//! The `cat` sub-command.
//!
//! Read a sequence of Cranelift IR files and print them again to stdout. This has the effect of
//! normalizing formatting and removing comments.

use crate::utils::read_to_string;
use anyhow::{Context, Result};
use cranelift_reader::parse_functions;

pub fn run(files: &[String]) -> Result<()> {
    for (i, f) in files.iter().enumerate() {
        if i != 0 {
            println!();
        }
        cat_one(f)?
    }
    Ok(())
}

fn cat_one(filename: &str) -> Result<()> {
    let buffer = read_to_string(&filename)?;
    let items =
        parse_functions(&buffer).with_context(|| format!("failed to parse {}", filename))?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!();
        }
        print!("{}", func);
    }

    Ok(())
}
