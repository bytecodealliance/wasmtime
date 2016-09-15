//! The `cat` sub-command.
//!
//! Read a sequence of Cretonne IL files and print them again to stdout. This has the effect of
//! normalizing formatting and removing comments.

use CommandResult;
use utils::read_to_string;
use cton_reader::parse_functions;

pub fn run(files: Vec<String>) -> CommandResult {
    for (i, f) in files.into_iter().enumerate() {
        if i != 0 {
            println!("");
        }
        try!(cat_one(f))
    }
    Ok(())
}

fn cat_one(filename: String) -> CommandResult {
    let buffer = try!(read_to_string(&filename).map_err(|e| format!("{}: {}", filename, e)));
    let items = try!(parse_functions(&buffer).map_err(|e| format!("{}: {}", filename, e)));

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!("");
        }
        print!("{}", func);
    }

    Ok(())
}
