//! CLI tool to use the functions provided by crates [wasm2cretonne](../wasm2cretonne/index.html)
//! and [wasmstandalone](../wasmstandalone/index.html).
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cretonne
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

extern crate wasm2cretonne;
extern crate wasmstandalone;
extern crate wasmparser;
extern crate cretonne;
extern crate wasmtext;
extern crate docopt;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate term;
extern crate tempdir;

use wasm2cretonne::{translate_module, TranslationResult, FunctionTranslation, DummyRuntime,
                    WasmRuntime};
use wasmstandalone::{StandaloneRuntime, compile_module, execute};
use std::path::PathBuf;
use wasmparser::{Parser, ParserState, WasmDecoder, SectionCode};
use wasmtext::Writer;
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
use std::io::{BufReader, stdout};
use std::io::prelude::*;
use docopt::Docopt;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;

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

const USAGE: &str = "
Wasm to Cretonne IL translation utility.
Takes a binary WebAssembly module and returns its functions in Cretonne IL format.
The translation is dependent on the runtime chosen.
The default is a dummy runtime that produces placeholder values.

Usage:
    wasm2cretonne-util [-vcop] <file>...
    wasm2cretonne-util -e [-mvcop] <file>...
    wasm2cretonne-util --help | --version

Options:
    -v, --verbose       displays info on the different steps
    -p, --print         displays the module and translated functions
    -c, --check         checks the corectness of the translated functions
    -o, --optimize      runs optimization passes on the translated functions
    -e, --execute       enable the standalone runtime and executes the start function of the module
    -m, --memory        interactive memory inspector after execution
    -h, --help          print this help message
    --version           print the Cretonne version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_verbose: bool,
    flag_execute: bool,
    flag_memory: bool,
    flag_check: bool,
    flag_optimize: bool,
    flag_print: bool,
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}


fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.help(true).version(Some(format!("0.0.0"))).deserialize())
        .unwrap_or_else(|e| e.exit());
    let mut terminal = term::stdout().unwrap();
    for filename in args.arg_file.iter() {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        match handle_module(&args, path.to_path_buf(), name) {
            Ok(()) => {}
            Err(message) => {
                terminal.fg(term::color::RED).unwrap();
                vprintln!(args.flag_verbose, "error");
                terminal.reset().unwrap();
                vprintln!(args.flag_verbose, "{}", message)
            }
        }
    }
}

fn handle_module(args: &Args, path: PathBuf, name: String) -> Result<(), String> {
    let mut terminal = term::stdout().unwrap();
    terminal.fg(term::color::YELLOW).unwrap();
    vprint!(args.flag_verbose, "Handling: ");
    terminal.reset().unwrap();
    vprintln!(args.flag_verbose, "\"{}\"", name);
    terminal.fg(term::color::MAGENTA).unwrap();
    vprint!(args.flag_verbose, "Translating...");
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
    let mut standalone_runtime = StandaloneRuntime::new();
    let translation = {
        let mut runtime: &mut WasmRuntime = if args.flag_execute {
            &mut standalone_runtime
        } else {
            &mut dummy_runtime
        };
        match translate_module(&data, runtime) {
            Ok(x) => x,
            Err(string) => {
                return Err(string);
            }
        }
    };
    terminal.fg(term::color::GREEN).unwrap();
    vprintln!(args.flag_verbose, " ok");
    terminal.reset().unwrap();
    if args.flag_check {
        terminal.fg(term::color::MAGENTA).unwrap();
        vprint!(args.flag_verbose, "Checking...   ");
        terminal.reset().unwrap();
        for func in translation.functions.iter() {
            let il = match func {
                &FunctionTranslation::Import() => continue,
                &FunctionTranslation::Code { ref il, .. } => il.clone(),
            };
            match verifier::verify_function(&il, None) {
                Ok(()) => (),
                Err(err) => return Err(pretty_verifier_error(&il, None, err)),
            }
        }
        terminal.fg(term::color::GREEN).unwrap();
        vprintln!(args.flag_verbose, " ok");
        terminal.reset().unwrap();
    }
    if args.flag_print {
        let mut writer1 = stdout();
        let mut writer2 = stdout();
        match pretty_print_translation(&name, &data, &translation, &mut writer1, &mut writer2) {
            Err(error) => return Err(String::from(error.description())),
            Ok(()) => (),
        }
    }
    if args.flag_optimize {
        terminal.fg(term::color::MAGENTA).unwrap();
        vprint!(args.flag_verbose, "Optimizing... ");
        terminal.reset().unwrap();
        for func in translation.functions.iter() {
            let mut il = match func {
                &FunctionTranslation::Import() => continue,
                &FunctionTranslation::Code { ref il, .. } => il.clone(),
            };
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
        vprintln!(args.flag_verbose, " ok");
        terminal.reset().unwrap();
    }
    if args.flag_execute {
        terminal.fg(term::color::MAGENTA).unwrap();
        vprint!(args.flag_verbose, "Compiling...   ");
        terminal.reset().unwrap();
        match compile_module(&translation) {
            Ok(exec) => {
                terminal.fg(term::color::GREEN).unwrap();
                vprintln!(args.flag_verbose, "ok");
                terminal.reset().unwrap();
                terminal.fg(term::color::MAGENTA).unwrap();
                vprint!(args.flag_verbose, "Executing...   ");
                terminal.reset().unwrap();
                match execute(exec) {
                    Ok(()) => {
                        terminal.fg(term::color::GREEN).unwrap();
                        vprintln!(args.flag_verbose, "ok");
                        terminal.reset().unwrap();
                    }
                    Err(s) => {
                        return Err(s);
                    }
                }
            }
            Err(s) => {
                return Err(s);
            }
        };
        if args.flag_memory {
            let mut input = String::new();
            terminal.fg(term::color::YELLOW).unwrap();
            println!("Inspecting memory");
            terminal.fg(term::color::MAGENTA).unwrap();
            println!("Type 'quit' to exit.");
            terminal.reset().unwrap();
            loop {
                input.clear();
                terminal.fg(term::color::YELLOW).unwrap();
                print!("Memory index, offset, length (e.g. 0,0,4): ");
                terminal.reset().unwrap();
                let _ = stdout().flush();
                match io::stdin().read_line(&mut input) {
                    Ok(_) => {
                        input.pop();
                        if input == "quit" {
                            break;
                        }
                        let split: Vec<&str> = input.split(",").collect();
                        if split.len() != 3 {
                            break;
                        }
                        let memory = standalone_runtime
                            .inspect_memory(str::parse(split[0]).unwrap(),
                                            str::parse(split[1]).unwrap(),
                                            str::parse(split[2]).unwrap());
                        let mut s = memory
                            .iter()
                            .fold(String::from("#"), |mut acc, byte| {
                                acc.push_str(format!("{:02x}_", byte).as_str());
                                acc
                            });
                        s.pop();
                        println!("{}", s);
                    }
                    Err(error) => return Err(String::from(error.description())),
                }
            }
        }
    }
    Ok(())
}

// Prints out a Wasm module, and for each function the corresponding translation in Cretonne IL.
fn pretty_print_translation(filename: &String,
                            data: &Vec<u8>,
                            translation: &TranslationResult,
                            writer_wast: &mut Write,
                            writer_cretonne: &mut Write)
                            -> Result<(), io::Error> {
    let mut terminal = term::stdout().unwrap();
    let mut parser = Parser::new(data.as_slice());
    let mut parser_writer = Writer::new(writer_wast);
    let imports_count = translation
        .functions
        .iter()
        .fold(0, |acc, &ref f| match f {
            &FunctionTranslation::Import() => acc + 1,
            &FunctionTranslation::Code { .. } => acc,
        });
    match parser.read() {
        s @ &ParserState::BeginWasm { .. } => parser_writer.write(&s)?,
        _ => panic!("modules should begin properly"),
    }
    loop {
        match parser.read() {
            s @ &ParserState::BeginSection { code: SectionCode::Code, .. } => {
                // The code section begins
                parser_writer.write(&s)?;
                break;
            }
            &ParserState::EndWasm => return Ok(()),
            s @ _ => parser_writer.write(&s)?,
        }
    }
    let mut function_index = 0;
    loop {
        match parser.read() {
            s @ &ParserState::BeginFunctionBody { .. } => {
                terminal.fg(term::color::BLUE).unwrap();
                write!(writer_cretonne,
                       "====== Function No. {} of module \"{}\" ======\n",
                       function_index,
                       filename)?;
                terminal.fg(term::color::CYAN).unwrap();
                write!(writer_cretonne, "Wast ---------->\n")?;
                terminal.reset().unwrap();
                parser_writer.write(&s)?;
            }
            s @ &ParserState::EndSection => {
                parser_writer.write(&s)?;
                break;
            }
            _ => panic!("wrong content in code section"),
        }
        {
            loop {
                match parser.read() {
                    s @ &ParserState::EndFunctionBody => {
                        parser_writer.write(&s)?;
                        break;
                    }
                    s @ _ => {
                        parser_writer.write(&s)?;
                    }
                };
            }
        }
        let mut function_string =
            format!("  {}",
                    match translation.functions[function_index + imports_count] {
                            FunctionTranslation::Code { ref il, .. } => il,
                            FunctionTranslation::Import() => panic!("should not happen"),
                        }
                        .display(None));
        function_string.pop();
        let function_str = str::replace(function_string.as_str(), "\n", "\n  ");
        terminal.fg(term::color::CYAN).unwrap();
        write!(writer_cretonne, "Cretonne IL --->\n")?;
        terminal.reset().unwrap();
        write!(writer_cretonne, "{}\n", function_str)?;
        function_index += 1;
    }
    loop {
        match parser.read() {
            &ParserState::EndWasm => return Ok(()),
            s @ _ => parser_writer.write(&s)?,
        }
    }
}

/// Pretty-print a verifier error.
pub fn pretty_verifier_error(func: &ir::Function,
                             isa: Option<&TargetIsa>,
                             err: verifier::Error)
                             -> String {
    let msg = err.to_string();
    let str1 = match err.location {
        AnyEntity::Inst(inst) => {
            format!("{}\n{}: {}\n\n",
                    msg,
                    inst,
                    func.dfg.display_inst(inst, isa))
        }
        _ => String::from(format!("{}\n", msg)),
    };
    format!("{}{}", str1, func.display(isa))
}
