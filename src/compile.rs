//! CLI tool to compile cretonne IL into native code.
//!
//! Reads IR files into Cretonne IL and compiles it.

use cton_reader::{parse_options, Location, parse_functions};
use std::path::PathBuf;
use cretonne::Context;
use cretonne::settings::{self, FlagsOrIsa};
use cretonne::isa::{self, TargetIsa};
use std::path::Path;
use utils::{pretty_error, read_to_string};

enum OwnedFlagsOrIsa {
    Flags(settings::Flags),
    Isa(Box<TargetIsa>),
}

pub fn run(
    files: Vec<String>,
    flag_print: bool,
    flag_set: Vec<String>,
    flag_isa: String,
) -> Result<(), String> {
    let mut flag_builder = settings::builder();
    parse_options(
        flag_set.iter().map(|x| x.as_str()),
        &mut flag_builder,
        &Location { line_number: 0 },
    ).map_err(|err| err.to_string())?;

    let mut words = flag_isa.trim().split_whitespace();
    // Look for `isa foo`.
    let owned_fisa = if let Some(isa_name) = words.next() {
        let isa_builder = isa::lookup(isa_name).map_err(|err| match err {
            isa::LookupError::Unknown => format!("unknown ISA '{}'", isa_name),
            isa::LookupError::Unsupported => format!("support for ISA '{}' not enabled", isa_name),
        })?;
        OwnedFlagsOrIsa::Isa(isa_builder.finish(settings::Flags::new(&flag_builder)))
    } else {
        OwnedFlagsOrIsa::Flags(settings::Flags::new(&flag_builder))
    };
    let fisa = match owned_fisa {
        OwnedFlagsOrIsa::Flags(ref flags) => FlagsOrIsa::from(flags),
        OwnedFlagsOrIsa::Isa(ref isa) => FlagsOrIsa::from(&**isa),
    };

    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(flag_print, path.to_path_buf(), name, &fisa)?;
    }
    Ok(())
}

fn handle_module(
    flag_print: bool,
    path: PathBuf,
    name: String,
    fisa: &FlagsOrIsa,
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
