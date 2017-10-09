//! CLI tool to use the functions provided by the [cretonne-wasm](../cton_wasm/index.html) crate.
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cretonne
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

use cton_wasm::{translate_module, DummyRuntime, WasmRuntime};
use std::path::PathBuf;
use cretonne::Context;
use cretonne::settings::FlagsOrIsa;
use std::fs::File;
use std::error::Error;
use std::io;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;
use term;
use utils::{pretty_verifier_error, pretty_error, parse_sets_and_isa, read_to_end};

macro_rules! vprintln {
    ($x: expr, $($tts:tt)*) => {
        if $x {
            println!($($tts)*);
        }
    }
}

macro_rules! vprint {
    ($x: expr, $($tts:tt)*) => {
        if $x {
            print!($($tts)*);
        }
    }
}

pub fn run(
    files: Vec<String>,
    flag_verbose: bool,
    flag_just_decode: bool,
    flag_check_translation: bool,
    flag_print: bool,
    flag_set: Vec<String>,
    flag_isa: String,
) -> Result<(), String> {
    let parsed = parse_sets_and_isa(flag_set, flag_isa)?;

    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        handle_module(
            flag_verbose,
            flag_just_decode,
            flag_check_translation,
            flag_print,
            path.to_path_buf(),
            name,
            parsed.as_fisa(),
        )?;
    }
    Ok(())
}

fn handle_module(
    flag_verbose: bool,
    flag_just_decode: bool,
    flag_check_translation: bool,
    flag_print: bool,
    path: PathBuf,
    name: String,
    fisa: FlagsOrIsa,
) -> Result<(), String> {
    let mut terminal = term::stdout().unwrap();
    terminal.fg(term::color::YELLOW).unwrap();
    vprint!(flag_verbose, "Handling: ");
    terminal.reset().unwrap();
    vprintln!(flag_verbose, "\"{}\"", name);
    terminal.fg(term::color::MAGENTA).unwrap();
    vprint!(flag_verbose, "Translating... ");
    terminal.reset().unwrap();
    let mut data = read_to_end(path.clone()).map_err(|err| {
        String::from(err.description())
    })?;
    if !data.starts_with(&[b'\0', b'a', b's', b'm']) {
        let tmp_dir = TempDir::new("cretonne-wasm").unwrap();
        let file_path = tmp_dir.path().join("module.wasm");
        File::create(file_path.clone()).unwrap();
        Command::new("wat2wasm")
            .arg(path.clone())
            .arg("-o")
            .arg(file_path.to_str().unwrap())
            .output()
            .or_else(|e| if let io::ErrorKind::NotFound = e.kind() {
                return Err(String::from("wat2wasm not found"));
            } else {
                return Err(String::from(e.description()));
            })?;
        data = read_to_end(file_path).map_err(
            |err| String::from(err.description()),
        )?;
    }
    let mut dummy_runtime = DummyRuntime::with_flags(fisa.flags.clone());
    let translation = translate_module(&data, &mut dummy_runtime)?;
    terminal.fg(term::color::GREEN).unwrap();
    vprintln!(flag_verbose, "ok");
    terminal.reset().unwrap();
    if flag_just_decode {
        return Ok(());
    }
    terminal.fg(term::color::MAGENTA).unwrap();
    if flag_check_translation {
        vprint!(flag_verbose, "Checking... ");
    } else {
        vprint!(flag_verbose, "Compiling... ");
    }
    terminal.reset().unwrap();
    let num_func_imports = dummy_runtime.get_num_func_imports();
    for (def_index, func) in translation.functions.iter().enumerate() {
        let func_index = num_func_imports + def_index;
        let mut context = Context::new();
        context.func = func.clone();
        if flag_check_translation {
            context.verify(fisa).map_err(|err| {
                pretty_verifier_error(&context.func, fisa.isa, err)
            })?;
            continue;
        }
        if let Some(isa) = fisa.isa {
            context.compile(isa).map_err(|err| {
                pretty_error(&context.func, fisa.isa, err)
            })?;
        } else {
            return Err(String::from("compilation requires a target isa"));
        }
        if flag_print {
            vprintln!(flag_verbose, "");
            if let Some(start_func) = dummy_runtime.start_func {
                if func_index == start_func {
                    println!("; Selected as wasm start function");
                }
            }
            for export_name in &dummy_runtime.functions[func_index].export_names {
                println!("; Exported as \"{}\"", export_name);
            }
            println!("{}", context.func.display(fisa.isa));
            vprintln!(flag_verbose, "");
        }
    }
    terminal.fg(term::color::GREEN).unwrap();
    vprintln!(flag_verbose, "ok");
    terminal.reset().unwrap();
    Ok(())
}
