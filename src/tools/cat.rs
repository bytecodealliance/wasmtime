//! The `cat` sub-command.
//!
//! Read a sequence of Cretonne IL files and print them again to stdout. This has the effect of
//! normalizing formatting and removing comments.

use CommandResult;
use std::fs::File;
use std::io::Read;

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
    let mut file = try!(File::open(&filename).map_err(|e| format!("{}: {}", filename, e)));
    let mut buffer = String::new();
    try!(file.read_to_string(&mut buffer)
        .map_err(|e| format!("Couldn't read {}: {}", filename, e)));

    print!("{}", buffer);
    Ok(())
}
