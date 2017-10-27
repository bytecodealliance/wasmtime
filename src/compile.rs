//! CLI tool to compile cretonne IL into native code.
//!
//! Reads IR files into Cretonne IL and compiles it.

use cton_reader::parse_functions;
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
    let items = parse_functions(&buffer).map_err(
        |e| format!("{}: {}", name, e),
    )?;
    for func in items.into_iter() {
        let mut context = Context::new();
        context.func = func;
        if let Some(isa) = fisa.isa {
            context.compile(isa).map_err(|err| {
                pretty_error(&context.func, fisa.isa, err)
            })?;
        } else {
            return Err(String::from("compilation requires a target isa"));
        }
        if flag_print {
            println!("{}", context.func.display(fisa.isa));
        }
    }
    Ok(())
}
