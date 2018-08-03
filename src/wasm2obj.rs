//! Translation from wasm to native object files.
//!
//! Reads a Wasm binary file, translates the functions' code to Cranelift
//! IL, then translates it to native code, and writes it out to a native
//! object file with relocations.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates, unstable_features)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        unicode_not_nfc, use_self
    )
)]

extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate docopt;
extern crate wasmtime_environ;
extern crate wasmtime_obj;
#[macro_use]
extern crate serde_derive;
extern crate faerie;

use cranelift_codegen::settings;
use docopt::Docopt;
use faerie::Artifact;
use std::error::Error;
use std::fmt::format;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use wasmtime_environ::{compile_module, Module, ModuleEnvironment};
use wasmtime_obj::emit_module;

const USAGE: &str = "
Wasm to native object translation utility.
Takes a binary WebAssembly module into a native object file.
The translation is dependent on the environment chosen.
The default is a dummy environment that produces placeholder values.

Usage:
    wasm2obj <file> -o <output>
    wasm2obj --help | --version

Options:
    -v, --verbose       displays the module and translated functions
    -h, --help          print this help message
    --version           print the Cranelift version
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
    let (flag_builder, isa_builder) = cranelift_native::builders().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let mut obj = Artifact::new(isa.triple().clone(), String::from(output));

    let mut module = Module::new();
    let environ = ModuleEnvironment::new(&*isa, &mut module);
    let translation = environ.translate(&data).map_err(|e| e.to_string())?;

    // FIXME: We need to initialize memory in a way that supports alternate
    // memory spaces, imported base addresses, and offsets.
    for init in &translation.lazy.data_initializers {
        obj.define("memory", Vec::from(init.data))
            .map_err(|err| format!("{}", err))?;
    }

    let (compilation, relocations) = compile_module(&translation, &*isa)?;

    emit_module(&mut obj, &translation.module, &compilation, &relocations)?;

    if !translation.module.tables.is_empty() {
        if translation.module.tables.len() > 1 {
            return Err(String::from("multiple tables not supported yet"));
        }
        return Err(String::from("FIXME: implement tables"));
    }

    // FIXME: Make the format a parameter.
    let file =
        ::std::fs::File::create(Path::new(output)).map_err(|x| format(format_args!("{}", x)))?;
    obj.write(file).map_err(|e| e.to_string())?;

    Ok(())
}
