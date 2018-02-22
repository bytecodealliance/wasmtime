//! Translation from wasm to native object files.
//!
//! Reads a Wasm binary file, translates the functions' code to Cretonne
//! IL, then translates it to native code, and writes it out to a native
//! object file with relocations.

extern crate cton_wasm;
extern crate wasmstandalone_obj;
extern crate wasmstandalone_runtime;
extern crate cretonne;
extern crate cton_native;
extern crate docopt;
#[macro_use]
extern crate serde_derive;
extern crate faerie;

use cton_wasm::translate_module;
use cretonne::settings;
use cretonne::isa;
use wasmstandalone_obj::emit_module;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use docopt::Docopt;
use std::path::Path;
use std::process;
use std::fmt::format;
use faerie::{Artifact, Elf, Target};

const USAGE: &str = "
Wasm to native object translation utility.
Takes a binary WebAssembly module into a native object file.
The translation is dependent on the runtime chosen.
The default is a dummy runtime that produces placeholder values.

Usage:
    wasm2obj <file> -o <output>
    wasm2obj --help | --version

Options:
    -v, --verbose       displays the module and translated functions
    -h, --help          print this help message
    --version           print the Cretonne version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: String,
    arg_output: String,
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
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

    let path = Path::new(&args.arg_file);
    match handle_module(path.to_path_buf(), &args.arg_output) {
        Ok(()) => {}
        Err(message) => {
            println!(" error: {}", message);
            process::exit(1);
        }
    }
}

fn handle_module(path: PathBuf, output: &str) -> Result<(), String> {
    let data = match read_wasm_file(path) {
        Ok(data) => data,
        Err(err) => {
            return Err(String::from(err.description()));
        }
    };

    // FIXME: Make the target a parameter.
    let (flag_builder, isa_builder) = cton_native::builders().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(&flag_builder));

    let mut module = wasmstandalone_runtime::Module::new();
    let mut environ = wasmstandalone_runtime::ModuleEnvironment::new(isa.flags(), &mut module);
    translate_module(&data, &mut environ)?;

    let mut obj = Artifact::new(faerie_target(&*isa)?, String::from(output));

    // FIXME: We need to initialize memory in a way that supports alternate
    // memory spaces, imported base addresses, and offsets.
    for init in &environ.lazy.data_initializers {
        obj.define("memory", Vec::from(init.data)).map_err(|err| {
            format!("{}", err)
        })?;
    }

    let translation = environ.finish_translation();

    let (compilation, relocations) = translation.compile(&*isa)?;

    emit_module(&mut obj, &compilation, &relocations)?;

    if !compilation.module.tables.is_empty() {
        if compilation.module.tables.len() > 1 {
            return Err(String::from("multiple tables not supported yet"));
        }
        return Err(String::from("FIXME: implement tables"));
    }

    // FIXME: Make the format a parameter.
    let file = ::std::fs::File::create(Path::new(output)).map_err(|x| {
        format(format_args!("{}", x))
    })?;
    obj.write::<Elf>(file).map_err(
        |x| format(format_args!("{}", x)),
    )?;

    Ok(())
}

fn faerie_target(isa: &isa::TargetIsa) -> Result<Target, String> {
    let name = isa.name();
    match name {
        "intel" => Ok(if isa.flags().is_64bit() {
            Target::X86_64
        } else {
            Target::X86
        }),
        "arm32" => Ok(Target::ARMv7),
        "arm64" => Ok(Target::ARM64),
        _ => Err(format!("unsupported isa: {}", name)),
    }
}
