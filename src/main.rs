//! CLI tool to use the functions provided by the [wasmtime](../wasmtime/index.html)
//! crate.
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cranelift
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate cranelift_wasm;
extern crate docopt;
extern crate wasmtime_execute;
extern crate wasmtime_runtime;
#[macro_use]
extern crate serde_derive;
extern crate tempdir;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_wasm::translate_module;
use docopt::Docopt;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::stdout;
use std::path::Path;
use std::path::PathBuf;
use std::process::{exit, Command};
use tempdir::TempDir;
use wasmtime_execute::{compile_module, execute};
use wasmtime_runtime::{Instance, Module, ModuleEnvironment};

const USAGE: &str = "
Wasm to Cranelift IL translation utility.
Takes a binary WebAssembly module and returns its functions in Cranelift IL format.
The translation is dependent on the environment chosen.

Usage:
    wasmtime [-mop] <file>...
    wasmtime --help | --version

Options:
    -o, --optimize      runs optimization passes on the translated functions
    -m, --memory        interactive memory inspector after execution
    -h, --help          print this help message
    --version           print the Cranelift version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_memory: bool,
    flag_optimize: bool,
}

fn read_to_end(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.help(true)
                .version(Some(String::from("0.0.0")))
                .deserialize()
        })
        .unwrap_or_else(|e| e.exit());
    let (mut flag_builder, isa_builder) = cranelift_native::builders().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    // Enable optimization if requested.
    if args.flag_optimize {
        flag_builder.set("opt_level", "best").unwrap();
    }

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    for filename in &args.arg_file {
        let path = Path::new(&filename);
        match handle_module(&args, path.to_path_buf(), &*isa) {
            Ok(()) => {}
            Err(message) => {
                let name = path.as_os_str().to_string_lossy();
                println!("error while processing {}: {}", name, message);
                exit(1);
            }
        }
    }
}

fn handle_module(args: &Args, path: PathBuf, isa: &TargetIsa) -> Result<(), String> {
    let mut data = read_to_end(path.clone()).map_err(|err| String::from(err.description()))?;
    if !data.starts_with(&[b'\0', b'a', b's', b'm']) {
        let tmp_dir = TempDir::new("cranelift-wasm").unwrap();
        let file_path = tmp_dir.path().join("module.wasm");
        File::create(file_path.clone()).unwrap();
        Command::new("wat2wasm")
            .arg(path.clone())
            .arg("-o")
            .arg(file_path.to_str().unwrap())
            .output()
            .or_else(|e| {
                if let io::ErrorKind::NotFound = e.kind() {
                    return Err(String::from("wat2wasm not found"));
                } else {
                    return Err(String::from(e.description()));
                }
            })?;
        data = read_to_end(file_path).map_err(|err| String::from(err.description()))?;
    }
    let mut module = Module::new();
    let mut environ = ModuleEnvironment::new(isa, &mut module);
    translate_module(&data, &mut environ).map_err(|e| e.to_string())?;
    let translation = environ.finish_translation();
    let instance = match compile_module(isa, &translation) {
        Ok(compilation) => {
            let mut instance =
                Instance::new(compilation.module, &translation.lazy.data_initializers);
            execute(&compilation, &mut instance)?;
            instance
        }
        Err(s) => {
            return Err(s);
        }
    };
    if args.flag_memory {
        let mut input = String::new();
        println!("Inspecting memory");
        println!("Type 'quit' to exit.");
        loop {
            input.clear();
            print!("Memory index, offset, length (e.g. 0,0,4): ");
            let _ = stdout().flush();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    input.pop();
                    if input == "quit" {
                        break;
                    }
                    let split: Vec<&str> = input.split(',').collect();
                    if split.len() != 3 {
                        break;
                    }
                    let memory = instance.inspect_memory(
                        str::parse(split[0]).unwrap(),
                        str::parse(split[1]).unwrap(),
                        str::parse(split[2]).unwrap(),
                    );
                    let mut s = memory.iter().fold(String::from("#"), |mut acc, byte| {
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
    Ok(())
}
