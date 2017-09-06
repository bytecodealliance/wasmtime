//! CLI tool to use the functions provided by crates [wasm2cretonne](../wasm2cretonne/index.html)
//! and [wasmstandalone](../wasmstandalone/index.html).
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cretonne
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

use cton_wasm::{translate_module, DummyRuntime, WasmRuntime};
use std::path::PathBuf;
use cretonne::loop_analysis::LoopAnalysis;
use cretonne::flowgraph::ControlFlowGraph;
use cretonne::dominator_tree::DominatorTree;
use cretonne::Context;
use cretonne::result::CtonError;
use cretonne::ir;
use cretonne::ir::entities::AnyEntity;
use cretonne::isa::TargetIsa;
use cretonne::verifier;
use std::fs::File;
use std::error::Error;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;
use term;

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


pub fn run(
    files: Vec<String>,
    flag_verbose: bool,
    flag_optimize: bool,
    flag_check: bool,
) -> Result<(), String> {
    for filename in files {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        match handle_module(
            flag_verbose,
            flag_optimize,
            flag_check,
            path.to_path_buf(),
            name,
        ) {
            Ok(()) => {}
            Err(message) => return Err(message),
        }
    }
    Ok(())
}

fn handle_module(
    flag_verbose: bool,
    flag_optimize: bool,
    flag_check: bool,
    path: PathBuf,
    name: String,
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
            return Err(String::from("the file extension is not wasm or wast"));
        }
        Some(ext) => {
            match ext.to_str() {
                Some("wasm") => {
                    match read_wasm_file(path.clone()) {
                        Ok(data) => data,
                        Err(err) => {
                            return Err(String::from(err.description()));
                        }
                    }
                }
                Some("wast") => {
                    let tmp_dir = TempDir::new("wasm2cretonne").unwrap();
                    let file_path = tmp_dir.path().join("module.wasm");
                    File::create(file_path.clone()).unwrap();
                    Command::new("wast2wasm")
                        .arg(path.clone())
                        .arg("-o")
                        .arg(file_path.to_str().unwrap())
                        .output()
                        .or_else(|e| if let io::ErrorKind::NotFound = e.kind() {
                            return Err(String::from("wast2wasm not found"));
                        } else {
                            return Err(String::from(e.description()));
                        })
                        .unwrap();
                    match read_wasm_file(file_path) {
                        Ok(data) => data,
                        Err(err) => {
                            return Err(String::from(err.description()));
                        }
                    }
                }
                None | Some(&_) => {
                    return Err(String::from("the file extension is not wasm or wast"));
                }
            }
        }
    };
    let mut dummy_runtime = DummyRuntime::new();
    let translation = {
        let runtime: &mut WasmRuntime = &mut dummy_runtime;
        match translate_module(&data, runtime) {
            Ok(x) => x,
            Err(string) => {
                return Err(string);
            }
        }
    };
    terminal.fg(term::color::GREEN).unwrap();
    vprintln!(flag_verbose, " ok");
    terminal.reset().unwrap();
    if flag_check {
        terminal.fg(term::color::MAGENTA).unwrap();
        vprint!(flag_verbose, "Checking...   ");
        terminal.reset().unwrap();
        for func in &translation.functions {
            match verifier::verify_function(func, None) {
                Ok(()) => (),
                Err(err) => return Err(pretty_verifier_error(func, None, err)),
            }
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
            let mut il = func.clone();
            let mut loop_analysis = LoopAnalysis::new();
            let mut cfg = ControlFlowGraph::new();
            cfg.compute(&il);
            let mut domtree = DominatorTree::new();
            domtree.compute(&mut il, &cfg);
            loop_analysis.compute(&mut il, &mut cfg, &mut domtree);
            let mut context = Context::new();
            context.func = il;
            context.cfg = cfg;
            context.domtree = domtree;
            context.loop_analysis = loop_analysis;
            match verifier::verify_context(&context.func, &context.cfg, &context.domtree, None) {
                Ok(()) => (),
                Err(err) => {
                    return Err(pretty_verifier_error(&context.func, None, err));
                }
            };
            match context.licm() {
                Ok(())=> (),
                Err(error) => {
                    match error {
                        CtonError::Verifier(err) => {
                            return Err(pretty_verifier_error(&context.func, None, err));
                        }
                        CtonError::InvalidInput |
                        CtonError::ImplLimitExceeded |
                        CtonError::CodeTooLarge => return Err(String::from(error.description())),
                    }
                }
            };
            match context.simple_gvn() {
                Ok(())=> (),
                Err(error) => {
                    match error {
                        CtonError::Verifier(err) => {
                            return Err(pretty_verifier_error(&context.func, None, err));
                        }
                        CtonError::InvalidInput |
                        CtonError::ImplLimitExceeded |
                        CtonError::CodeTooLarge => return Err(String::from(error.description())),
                    }
                }
            };
            match verifier::verify_context(&context.func, &context.cfg, &context.domtree, None) {
                Ok(()) => (),
                Err(err) => return Err(pretty_verifier_error(&context.func, None, err)),
            }
        }
        terminal.fg(term::color::GREEN).unwrap();
        vprintln!(flag_verbose, " ok");
        terminal.reset().unwrap();
    }
    Ok(())
}

/// Pretty-print a verifier error.
pub fn pretty_verifier_error(
    func: &ir::Function,
    isa: Option<&TargetIsa>,
    err: verifier::Error,
) -> String {
    let msg = err.to_string();
    let str1 = match err.location {
        AnyEntity::Inst(inst) => {
            format!(
                "{}\n{}: {}\n\n",
                msg,
                inst,
                func.dfg.display_inst(inst, isa)
            )
        }
        _ => String::from(format!("{}\n", msg)),
    };
    format!("{}{}", str1, func.display(isa))
}
