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
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::new_without_default, clippy::new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

#[macro_use]
extern crate serde_derive;

use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_native;
use docopt::Docopt;
use file_per_thread_logger;
use pretty_env_logger;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::exit;
use wabt;
use wasmtime_jit::{ActionOutcome, Context};
use wasmtime_wast::instantiate_spectest;

static LOG_FILENAME_PREFIX: &str = "wasmtime.dbg.";

const USAGE: &str = "
Wasm runner.

Takes a binary (wasm) or text (wat) WebAssembly module and instantiates it,
including calling the start function if one is present. Additional functions
given with --invoke are then called.

Usage:
    wasmtime [-od] <file>...
    wasmtime [-od] <file>... --invoke=<fn>
    wasmtime --help | --version

Options:
    --invoke=<fn>       name of function to run
    -o, --optimize      runs optimization passes on the translated functions
    -d, --debug         enable debug output on stderr/stdout
    -h, --help          print this help message
    --version           print the Cranelift version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_optimize: bool,
    flag_debug: bool,
    flag_invoke: Option<String>,
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

    if args.flag_debug {
        pretty_env_logger::init();
    } else {
        file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
    }

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let mut flag_builder = settings::builder();

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    // Enable optimization if requested.
    if args.flag_optimize {
        flag_builder.set("opt_level", "best").unwrap();
    }

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut context = Context::with_isa(isa);

    // Make spectest available by default.
    context.instance(
        Some("spectest".to_owned()),
        instantiate_spectest().expect("instantiating spectest"),
    );

    for filename in &args.arg_file {
        let path = Path::new(&filename);
        match handle_module(&mut context, &args, path) {
            Ok(()) => {}
            Err(message) => {
                let name = path.as_os_str().to_string_lossy();
                println!("error while processing {}: {}", name, message);
                exit(1);
            }
        }
    }
}

fn handle_module(context: &mut Context, args: &Args, path: &Path) -> Result<(), String> {
    let mut data =
        read_to_end(path.to_path_buf()).map_err(|err| String::from(err.description()))?;

    // If data is using wat-format, first convert data to wasm.
    if !data.starts_with(&[b'\0', b'a', b's', b'm']) {
        data = wabt::wat2wasm(data).map_err(|err| String::from(err.description()))?;
    }

    // Create a new `Instance` by compiling and instantiating a wasm module.
    let index = context
        .instantiate_module(None, &data)
        .map_err(|e| e.to_string())?;

    // If a function to invoke was given, invoke it.
    if let Some(ref f) = args.flag_invoke {
        match context
            .invoke_indexed(index, f, &[])
            .map_err(|e| e.to_string())?
        {
            ActionOutcome::Returned { .. } => {}
            ActionOutcome::Trapped { message } => {
                return Err(format!("Trap from within function {}: {}", f, message));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use cranelift_codegen::settings;
    use cranelift_codegen::settings::Configurable;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::rc::Rc;
    use wabt;
    use wasmtime_jit::{instantiate, Compiler, NullResolver};

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

        let mut resolver = NullResolver {};
        let mut compiler = Compiler::new(isa);
        let global_exports = Rc::new(RefCell::new(HashMap::new()));
        let instance = instantiate(&mut compiler, &data, &mut resolver, global_exports);
        assert!(instance.is_ok());
    }
}
