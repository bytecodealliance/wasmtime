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
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
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
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::process::exit;
use wabt;
use wasmtime_jit::{ActionOutcome, Context};
use wasmtime_wasi::instantiate_wasi;
use wasmtime_wast::instantiate_spectest;

static LOG_FILENAME_PREFIX: &str = "wasmtime.dbg.";

const USAGE: &str = "
Wasm runner.

Takes a binary (wasm) or text (wat) WebAssembly module and instantiates it,
including calling the start function if one is present. Additional functions
given with --invoke are then called.

Usage:
    wasmtime [-odg] [--preload=<wasm>...] [--env=<env>...] [--dir=<dir>...] [--mapdir=<mapping>...] <file> [<arg>...]
    wasmtime [-odg] [--preload=<wasm>...] [--env=<env>...] [--dir=<dir>...] [--mapdir=<mapping>...] --invoke=<fn> <file> [<arg>...]
    wasmtime --help | --version

Options:
    --invoke=<fn>       name of function to run
    -o, --optimize      runs optimization passes on the translated functions
    -g                  generate debug information
    -d, --debug         enable debug output on stderr/stdout
    --preload=<wasm>    load an additional wasm module before loading the main module
    --env=<env>         pass an environment variable (\"key=value\") to the program
    --dir=<dir>         grant access to the given host directory
    --mapdir=<mapping>  where <mapping> has the form <wasmdir>:<hostdir>, grant access to
                        the given host directory with the given wasm directory name
    -h, --help          print this help message
    --version           print the Cranelift version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: String,
    arg_arg: Vec<String>,
    flag_optimize: bool,
    flag_debug: bool,
    flag_g: bool,
    flag_invoke: Option<String>,
    flag_preload: Vec<String>,
    flag_env: Vec<String>,
    flag_dir: Vec<String>,
    flag_mapdir: Vec<String>,
}

fn read_to_end(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn read_wasm(path: PathBuf) -> Result<Vec<u8>, String> {
    let data = read_to_end(path).map_err(|err| err.to_string())?;

    // If data is a wasm binary, use that. If it's using wat format, convert it
    // to a wasm binary with wat2wasm.
    Ok(if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        data
    } else {
        wabt::wat2wasm(data).map_err(|err| String::from(err.description()))?
    })
}

fn compute_preopen_dirs(flag_dir: &[String], flag_mapdir: &[String]) -> Vec<(String, File)> {
    let mut preopen_dirs = Vec::new();

    for dir in flag_dir {
        let preopen_dir = File::open(dir).unwrap_or_else(|err| {
            println!("error while pre-opening directory {}: {}", dir, err);
            exit(1);
        });
        preopen_dirs.push((dir.clone(), preopen_dir));
    }

    for mapdir in flag_mapdir {
        let parts: Vec<&str> = mapdir.split(':').collect();
        if parts.len() != 2 {
            println!("--mapdir argument must contain exactly one colon, separating a guest directory name and a host directory name");
            exit(1);
        }
        let (key, value) = (parts[0], parts[1]);
        let preopen_dir = File::open(value).unwrap_or_else(|err| {
            println!("error while pre-opening directory {}: {}", value, err);
            exit(1);
        });
        preopen_dirs.push((key.to_string(), preopen_dir));
    }

    preopen_dirs
}

/// Compute the argv array values.
fn compute_argv(argv0: &str, arg_arg: &[String]) -> Vec<String> {
    let mut result = Vec::new();

    // Add argv[0], which is the program name. Only include the base name of the
    // main wasm module, to avoid leaking path information.
    result.push(
        Path::new(argv0)
            .components()
            .next_back()
            .map(Component::as_os_str)
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_owned(),
    );

    // Add the remaining arguments.
    for arg in arg_arg {
        result.push(arg.to_owned());
    }

    result
}

/// Compute the environ array values.
fn compute_environ(flag_env: &[String]) -> Vec<(String, String)> {
    let mut result = Vec::new();

    // Add the environment variables, which must be of the form "key=value".
    for env in flag_env {
        let split = env.splitn(2, '=').collect::<Vec<_>>();
        if split.len() != 2 {
            println!(
                "environment variables must be of the form \"key=value\"; got \"{}\"",
                env
            );
        }
        result.push((split[0].to_owned(), split[1].to_owned()));
    }

    result
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
    context.name_instance(
        "spectest".to_owned(),
        instantiate_spectest().expect("instantiating spectest"),
    );

    // Make wasi available by default.
    let global_exports = context.get_global_exports();
    let preopen_dirs = compute_preopen_dirs(&args.flag_dir, &args.flag_mapdir);
    let argv = compute_argv(&args.arg_file, &args.arg_arg);
    let environ = compute_environ(&args.flag_env);
    context.name_instance(
        "wasi_unstable".to_owned(),
        instantiate_wasi("", global_exports, &preopen_dirs, &argv, &environ)
            .expect("instantiating wasi"),
    );

    // FIXME: Also recognize "env", for compatibility with clang/llvm 8.0. And use
    // "__wasi_" prefixes for compatibility with prototype reference-sysroot.
    let global_exports = context.get_global_exports();
    context.name_instance(
        "env".to_owned(),
        instantiate_wasi("__wasi_", global_exports, &preopen_dirs, &argv, &environ)
            .expect("instantiating wasi"),
    );

    // Enable/disable producing of debug info.
    context.set_debug_info(args.flag_g);

    // Load the preload wasm modules.
    for filename in &args.flag_preload {
        let path = Path::new(&filename);
        match handle_module(&mut context, &args, path) {
            Ok(()) => {}
            Err(message) => {
                let name = path.as_os_str().to_string_lossy();
                println!("error while processing preload {}: {}", name, message);
                exit(1);
            }
        }
    }

    // Load the main wasm module.
    let path = Path::new(&args.arg_file);
    match handle_module(&mut context, &args, path) {
        Ok(()) => {}
        Err(message) => {
            let name = path.as_os_str().to_string_lossy();
            println!("error while processing main module {}: {}", name, message);
            exit(1);
        }
    }
}

fn handle_module(context: &mut Context, args: &Args, path: &Path) -> Result<(), String> {
    // Read the wasm module binary.
    let data = read_wasm(path.to_path_buf())?;

    // Compile and instantiating a wasm module.
    let mut instance = context
        .instantiate_module(None, &data)
        .map_err(|e| e.to_string())?;

    // If a function to invoke was given, invoke it.
    if let Some(ref f) = args.flag_invoke {
        match context
            .invoke(&mut instance, f, &[])
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
