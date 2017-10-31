//! CLI tool to compile cretonne IL into native code.
//!
//! Reads IR files into Cretonne IL and compiles it.

use cton_reader::parse_test;
use std::path::PathBuf;
use cretonne::Context;
use cretonne::settings::FlagsOrIsa;
use std::path::Path;
use utils::{pretty_error, read_to_string, parse_sets_and_isa};

pub fn run(
    files: Vec<String>,
    flag_print: bool,
    mut flag_set: Vec<String>,
    flag_isa: String,
) -> Result<(), String> {
    // Enable the verifier by default, since we're reading IL in from a
    // text file.
    flag_set.insert(0, "enable_verifier=1".to_string());

    let parsed = parse_sets_and_isa(flag_set, flag_isa)?;

    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(flag_print, path.to_path_buf(), name, parsed.as_fisa())?;
    }
    Ok(())
}

fn handle_module(
    flag_print: bool,
    path: PathBuf,
    name: String,
    fisa: FlagsOrIsa,
) -> Result<(), String> {
    let buffer = read_to_string(&path).map_err(
        |e| format!("{}: {}", name, e),
    )?;
    let test_file = parse_test(&buffer).map_err(|e| format!("{}: {}", name, e))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contins a unique isa, use that.
    let isa = if let Some(isa) = fisa.isa {
        isa
    } else if let Some(isa) = test_file.isa_spec.unique_isa() {
        isa
    } else {
        return Err(String::from("compilation requires a target isa"));
    };

    for (func, _) in test_file.functions.into_iter() {
        let mut context = Context::new();
        context.func = func;
        context.compile(isa).map_err(|err| {
            pretty_error(&context.func, Some(isa), err)
        })?;
        if flag_print {
            println!("{}", context.func.display(isa));
        }
    }

    Ok(())
}
