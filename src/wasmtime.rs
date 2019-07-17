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

use docopt::Docopt;
use libwasmtime::handle_module;
use pretty_env_logger;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Component;
use std::path::Path;
use std::process::exit;
use wasi_common::preopen_dir;
use wasmtime_environ::cache_conf;
use wasmtime_wasi::instantiate_wasi;
use wasmtime_wast::instantiate_spectest;

#[cfg(feature = "wasi-c")]
use wasmtime_wasi_c::instantiate_wasi_c;

mod utils;

static LOG_FILENAME_PREFIX: &str = "wasmtime.dbg.";

const USAGE: &str = "
Wasm runner.

Takes a binary (wasm) or text (wat) WebAssembly module and instantiates it,
including calling the start function if one is present. Additional functions
given with --invoke are then called.

Usage:
    wasmtime [-ocdg] [--wasi-c] [--preload=<wasm>...] [--env=<env>...] [--dir=<dir>...] [--mapdir=<mapping>...] <file> [<arg>...]
    wasmtime [-ocdg] [--wasi-c] [--preload=<wasm>...] [--env=<env>...] [--dir=<dir>...] [--mapdir=<mapping>...] --invoke=<fn> <file> [<arg>...]
    wasmtime --help | --version

Options:
    --invoke=<fn>       name of function to run
    -o, --optimize      runs optimization passes on the translated functions
    -c, --cache         enable caching system
    -g                  generate debug information
    -d, --debug         enable debug output on stderr/stdout
    --wasi-c            enable the wasi-c implementation of WASI
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
    flag_cache: bool,
    flag_debug: bool,
    flag_g: bool,
    flag_invoke: Option<String>,
    flag_preload: Vec<String>,
    flag_env: Vec<String>,
    flag_dir: Vec<String>,
    flag_mapdir: Vec<String>,
    flag_wasi_c: bool,
}

fn compute_preopen_dirs(flag_dir: &[String], flag_mapdir: &[String]) -> Vec<(String, File)> {
    let mut preopen_dirs = Vec::new();

    for dir in flag_dir {
        let preopen_dir = preopen_dir(dir).unwrap_or_else(|err| {
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
        let preopen_dir = preopen_dir(value).unwrap_or_else(|err| {
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
    let args: Args = {
        let version = env!("CARGO_PKG_VERSION");
        Docopt::new(USAGE)
            .and_then(|d| {
                d.help(true)
                    .version(Some(String::from(version)))
                    .deserialize()
            })
            .unwrap_or_else(|e| e.exit())
    };

    if args.flag_debug {
        pretty_env_logger::init();
    } else {
        utils::init_file_per_thread_logger();
    }

    cache_conf::init(args.flag_cache);

    let mut context = libwasmtime::ContextBuilder {
        // Enable optimization if requested.
        opt_level: if args.flag_optimize {
            Some("best")
        } else {
            None
        },

        // Enable verification if requested
        enable_verifier: cfg!(debug_assertions),

        // Enable/disable producing of debug info.
        set_debug_info: args.flag_g,
    }
    .try_build()
    .expect("couldn't build Context");

    for (name, instance) in vec![
        // Make spectest available by default.
        (
            "spectest".to_owned(),
            instantiate_spectest().expect("instantiating spectest"),
        ),
        // Make wasi available by default.
        ("wasi_unstable".to_owned(), {
            let wasi_instantiation_fn = if args.flag_wasi_c {
                #[cfg(feature = "wasi-c")]
                {
                    instantiate_wasi_c
                }
                #[cfg(not(feature = "wasi-c"))]
                {
                    panic!("wasi-c feature not enabled at build time")
                }
            } else {
                instantiate_wasi
            };

            let global_exports = context.get_global_exports();
            let preopen_dirs = compute_preopen_dirs(&args.flag_dir, &args.flag_mapdir);
            let argv = compute_argv(&args.arg_file, &args.arg_arg);
            let environ = compute_environ(&args.flag_env);

            wasi_instantiation_fn("", global_exports, &preopen_dirs, &argv, &environ)
                .expect("instantiating wasi")
        }),
    ] {
        context.name_instance(name, instance);
    }

    // Load the preload wasm modules.
    for filename in &args.flag_preload {
        let module = File::open(&filename).expect(&format!("can't open module at {}", filename));
        match handle_module(&mut context, module, args.flag_invoke.as_ref()) {
            Ok(()) => {}
            Err(message) => {
                println!("error while processing preload {}: {}", filename, message);
                exit(1);
            }
        }
    }

    // Load the main wasm module.
    let module =
        File::open(&args.arg_file).expect(&format!("can't open module at {}", &args.arg_file));
    let flag_invoke = None;
    match handle_module(&mut context, module, flag_invoke) {
        Ok(()) => {}
        Err(message) => {
            println!(
                "error while processing main module {}: {}",
                args.arg_file, message
            );
            exit(1);
        }
    }
}
