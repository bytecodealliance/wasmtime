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
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;
use term;
use utils::{pretty_verifier_error, pretty_error, parse_sets_and_isa};

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

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
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
    let data = match path.extension() {
        None => {
            return Err(String::from("the file extension is not wasm or wat"));
        }
        Some(ext) => {
            match ext.to_str() {
                Some("wasm") => {
                    read_wasm_file(path.clone()).map_err(|err| {
                        String::from(err.description())
                    })?
                }
                Some("wat") => {
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
                    read_wasm_file(file_path).map_err(|err| {
                        String::from(err.description())
                    })?
                }
                None | Some(&_) => {
                    return Err(String::from("the file extension is not wasm or wat"));
                }
            }
        }
    };
    let mut dummy_runtime = DummyRuntime::with_flags(fisa.flags.clone());
    let translation = {
        let runtime: &mut WasmRuntime = &mut dummy_runtime;
        translate_module(&data, runtime)?
    };
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
    for func in &translation.functions {
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
            println!("{}", context.func.display(fisa.isa));
            vprintln!(flag_verbose, "");
        }
    }
    terminal.fg(term::color::GREEN).unwrap();
    vprintln!(flag_verbose, "ok");
    terminal.reset().unwrap();
    Ok(())
}
