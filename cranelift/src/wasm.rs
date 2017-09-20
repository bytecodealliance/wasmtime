//! CLI tool to use the functions provided by the [cretonne-wasm](../cton_wasm/index.html) crate.
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cretonne
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

use cton_wasm::{translate_module, DummyRuntime, WasmRuntime};
use cton_reader::{parse_options, Location};
use std::path::PathBuf;
use cretonne::Context;
use cretonne::verifier;
use cretonne::settings::{self, FlagsOrIsa};
use cretonne::isa::{self, TargetIsa};
use std::fs::File;
use std::error::Error;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;
use term;
use utils::{pretty_verifier_error, pretty_error};

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
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}

enum OwnedFlagsOrIsa {
    Flags(settings::Flags),
    Isa(Box<TargetIsa>),
}

pub fn run(
    files: Vec<String>,
    flag_verbose: bool,
    flag_optimize: bool,
    flag_check: bool,
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
        handle_module(
            flag_verbose,
            flag_optimize,
            flag_check,
            path.to_path_buf(),
            name,
            &fisa,
        )?;
    }
    Ok(())
}

fn handle_module(
    flag_verbose: bool,
    flag_optimize: bool,
    flag_check: bool,
    path: PathBuf,
    name: String,
    fisa: &FlagsOrIsa,
) -> Result<(), String> {
    let mut terminal = term::stdout().unwrap();
    terminal.fg(term::color::YELLOW).unwrap();
    vprint!(flag_verbose, "Handling: ");
    terminal.reset().unwrap();
    vprintln!(flag_verbose, "\"{}\"", name);
    terminal.fg(term::color::MAGENTA).unwrap();
    vprint!(flag_verbose, "Translating...");
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
                        })
                        .unwrap();
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
    vprintln!(flag_verbose, " ok");
    terminal.reset().unwrap();
    if flag_check {
        terminal.fg(term::color::MAGENTA).unwrap();
        vprint!(flag_verbose, "Checking...   ");
        terminal.reset().unwrap();
        for func in &translation.functions {
            verifier::verify_function(func, *fisa).map_err(|err| {
                pretty_verifier_error(func, fisa.isa, err)
            })?;
        }
        terminal.fg(term::color::GREEN).unwrap();
        vprintln!(flag_verbose, " ok");
        terminal.reset().unwrap();
    }
    if flag_optimize {
        terminal.fg(term::color::MAGENTA).unwrap();
        vprint!(flag_verbose, "Optimizing... ");
        terminal.reset().unwrap();
        for func in &translation.functions {
            let mut context = Context::new();
            context.func = func.clone();
            context.verify(*fisa).map_err(|err| {
                pretty_verifier_error(&context.func, fisa.isa, err)
            })?;
            context.flowgraph();
            context.compute_loop_analysis();
            context.licm(*fisa).map_err(|err| {
                pretty_error(&context.func, fisa.isa, err)
            })?;
            context.simple_gvn(*fisa).map_err(|err| {
                pretty_error(&context.func, fisa.isa, err)
            })?;
        }
        terminal.fg(term::color::GREEN).unwrap();
        vprintln!(flag_verbose, " ok");
        terminal.reset().unwrap();
    }
    Ok(())
}
