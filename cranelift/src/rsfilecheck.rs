//! The `filecheck` sub-command.
//!
//! This file is named to avoid a name collision with the filecheck crate.

use CommandResult;
use filecheck::{Checker, CheckerBuilder, NO_VARIABLES};
use std::io::{self, Read};
use utils::read_to_string;

pub fn run(files: &[String], verbose: bool) -> CommandResult {
    if files.is_empty() {
        return Err("No check files".to_string());
    }
    let checker = read_checkfile(&files[0])?;
    if checker.is_empty() {
        return Err(format!("{}: no filecheck directives found", files[0]));
    }

    // Print out the directives under --verbose.
    if verbose {
        println!("{}", checker);
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|e| format!("stdin: {}", e))?;

    if verbose {
        let (success, explain) = checker
            .explain(&buffer, NO_VARIABLES)
            .map_err(|e| e.to_string())?;
        print!("{}", explain);
        if success {
            println!("OK");
            Ok(())
        } else {
            Err("Check failed".to_string())
        }
    } else if checker
        .check(&buffer, NO_VARIABLES)
        .map_err(|e| e.to_string())?
    {
        Ok(())
    } else {
        let (_, explain) = checker
            .explain(&buffer, NO_VARIABLES)
            .map_err(|e| e.to_string())?;
        print!("{}", explain);
        Err("Check failed".to_string())
    }
}

fn read_checkfile(filename: &str) -> Result<Checker, String> {
    let buffer = read_to_string(&filename).map_err(|e| format!("{}: {}", filename, e))?;
    let mut builder = CheckerBuilder::new();
    builder
        .text(&buffer)
        .map_err(|e| format!("{}: {}", filename, e))?;
    Ok(builder.finish())
}
