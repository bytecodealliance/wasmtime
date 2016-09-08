use CommandResult;
use filecheck::{CheckerBuilder, Checker, NO_VARIABLES};
use std::fs::File;
use std::io::{self, Read};

pub fn run(files: Vec<String>, verbose: bool) -> CommandResult {
    if files.is_empty() {
        return Err("No check files".to_string());
    }
    let checker = try!(read_checkfile(&files[0]));
    if checker.is_empty() {
        return Err(format!("{}: no filecheck directives found", files[0]));
    }

    // Print out the directives under --verbose.
    if verbose {
        println!("{}", checker);
    }

    let mut buffer = String::new();
    try!(io::stdin().read_to_string(&mut buffer).map_err(|e| format!("stdin: {}", e)));

    if try!(checker.check(&buffer, NO_VARIABLES).map_err(|e| e.to_string())) {
        Ok(())
    } else {
        // TODO: We need to do better than this.
        Err("Check failed".to_string())
    }
}

fn read_checkfile(filename: &str) -> Result<Checker, String> {
    let mut file = try!(File::open(&filename).map_err(|e| format!("{}: {}", filename, e)));
    let mut buffer = String::new();
    try!(file.read_to_string(&mut buffer)
        .map_err(|e| format!("Couldn't read {}: {}", filename, e)));

    let mut builder = CheckerBuilder::new();
    try!(builder.text(&buffer).map_err(|e| format!("{}: {}", filename, e)));
    Ok(builder.finish())
}
