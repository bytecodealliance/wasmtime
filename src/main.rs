//! CLI tool to use the functions provided by the [wasmtime](../wasmtime/index.html)
//! crate.
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cranelift
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(
    feature = "clippy",
    plugin(clippy(conf_file = "../../clippy.toml"))
)]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(new_without_default, new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic,
        mut_mut,
        nonminimal_bool,
        option_map_unwrap_or,
        option_map_unwrap_or_else,
        unicode_not_nfc,
        use_self
    )
)]

extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_native;
extern crate cranelift_wasm;
extern crate docopt;
extern crate wasmtime_environ;
extern crate wasmtime_execute;
#[macro_use]
extern crate serde_derive;
extern crate file_per_thread_logger;
extern crate pretty_env_logger;
extern crate wabt;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_entity::EntityRef;
use cranelift_wasm::MemoryIndex;
use docopt::Docopt;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::stdout;
use std::path::Path;
use std::path::PathBuf;
use std::process::{exit, Command};
use wasmtime_environ::{Module, ModuleEnvironment, Tunables};
use wasmtime_execute::{compile_and_link_module, execute, finish_instantiation, Instance};

static LOG_FILENAME_PREFIX: &str = "cranelift.dbg.";

const USAGE: &str = "
Wasm to Cranelift IL translation utility.
Takes a binary WebAssembly module and returns its functions in Cranelift IL format.
The translation is dependent on the environment chosen.

Usage:
    wasmtime [-mopd] <file>...
    wasmtime [-mopd] <file>... --function=<fn>
    wasmtime --help | --version

Options:
    -o, --optimize      runs optimization passes on the translated functions
    -m, --memory        interactive memory inspector after execution
    --function=<fn>     name of function to run
    -h, --help          print this help message
    --version           print the Cranelift version
    -d, --debug         enable debug output on stderr/stdout
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_memory: bool,
    flag_optimize: bool,
    flag_debug: bool,
    flag_function: Option<String>,
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
        }).unwrap_or_else(|e| e.exit());
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let mut flag_builder = settings::builder();

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    if args.flag_debug {
        pretty_env_logger::init();
    } else {
        file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
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
    // if data is using wat-format, first convert data to wasm
    if !data.starts_with(&[b'\0', b'a', b's', b'm']) {
        data = wabt::wat2wasm(data).map_err(|err| String::from(err.description()))?;
    }
    let mut module = Module::new();
    // TODO: Expose the tunables as command-line flags.
    let tunables = Tunables::default();
    let environ = ModuleEnvironment::new(isa, &mut module, tunables);

    let imports_resolver = |_env: &str, _function: &str| None;

    let translation = environ.translate(&data).map_err(|e| e.to_string())?;

    let instance = match compile_and_link_module(isa, &translation, &imports_resolver) {
        Ok(compilation) => {
            let mut instance = Instance::new(
                translation.module,
                &compilation,
                &translation.lazy.data_initializers,
            )?;

            let mut context =
                finish_instantiation(&translation.module, &compilation, &mut instance)?;

            if let Some(ref f) = args.flag_function {
                execute(&translation.module, &compilation, &mut context, &f)?;
            }

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
                        MemoryIndex::new(str::parse(split[0]).unwrap()),
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

#[cfg(test)]
mod tests {
    use cranelift_codegen::settings;
    use cranelift_codegen::settings::Configurable;
    use std::path::PathBuf;
    use wabt;
    use wasmtime_environ::{Module, ModuleEnvironment, Tunables};

    const PATH_MODULE_RS2WASM_ADD_FUNC: &str = r"filetests/rs2wasm-add-func.wat";

    /// Simple test reading a wasm-file and translating to binary representation.
    #[test]
    fn test_environ_translate() {
        let path = PathBuf::from(PATH_MODULE_RS2WASM_ADD_FUNC);
        let wat_data = super::read_to_end(path).unwrap();
        assert!(wat_data.len() > 0);

        let data = wabt::wat2wasm(wat_data).expect("expecting valid wat-file");
        assert!(data.len() > 0);

        let mut flag_builder = settings::builder();
        flag_builder.enable("enable_verifier").unwrap();

        let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
            panic!("host machine is not a supported target");
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder));

        let mut module = Module::new();
        let tunables = Tunables::default();
        let environ = ModuleEnvironment::new(&*isa, &mut module, tunables);

        let translation = environ.translate(&data);
        assert!(translation.is_ok());
    }
}
