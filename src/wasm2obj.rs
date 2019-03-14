//! Translation from wasm to native object files.
//!
//! Reads a Wasm binary file, translates the functions' code to Cranelift
//! IL, then translates it to native code, and writes it out to a native
//! object file with relocations.

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

use clap::{App, Arg};
use cranelift_codegen::isa;
use cranelift_codegen::settings;
use cranelift_native;
use faerie::Artifact;
use std::error::Error;
use std::fmt::format;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::str;
use std::str::FromStr;
use target_lexicon::Triple;
use wasmtime_debug::{emit_debugsections, read_debuginfo};
use wasmtime_environ::{cranelift, ModuleEnvironment, Tunables};
use wasmtime_obj::emit_module;

#[derive(Debug, Clone)]
struct Args {
    arg_file: String,
    arg_output: String,
    arg_target: Option<String>,
    flag_g: bool,
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() {
    let cli = App::new("Wasm to native object translation utility")
                .version("0.0.0")
                .about("Takes a binary WebAssembly module into a native object file. The translation is dependent on the environment chosen. The default is a dummy environment that produces placeholder values.")
                .arg(Arg::with_name("verbose")
                    .short("v")
                    .long("verbose")
                    .help("displays the module and translated functions"))
                .arg(Arg::with_name("target")
                    .short("t")
                    .long("target")
                    .value_name("TARGET")
                    .help("build for the target triple; default is the host machines")
                    .takes_value(true))
                .arg(Arg::with_name("debug")
                    .short("g")
                    .long("debug")
                    .help("generate debug information"))
                .arg(Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .value_name("OUTPUT")
                    .help("output location")
                    .required(true)
                    .takes_value(true))
                .arg(Arg::with_name("input")
                    .help("Sets the input file to use")
                    .required(true)
                    .index(1))
                .get_matches();

    let args: Args = Args {
        arg_file: clap::value_t!(cli.value_of("input"), String).unwrap(),
        arg_output: cli.value_of("output").unwrap().to_string(),
        arg_target: match cli.value_of("target") {
            Some(x) => Some(x.to_string()),
            None => None,
        },
        flag_g: cli.is_present("debug"),
    };

    let path = Path::new(&args.arg_file);
    match handle_module(
        path.to_path_buf(),
        &args.arg_target,
        &args.arg_output,
        args.flag_g,
    ) {
        Ok(()) => {}
        Err(message) => {
            println!(" error: {}", message);
            process::exit(1);
        }
    }
}

fn handle_module(
    path: PathBuf,
    target: &Option<String>,
    output: &str,
    generate_debug_info: bool,
) -> Result<(), String> {
    let data = match read_wasm_file(path) {
        Ok(data) => data,
        Err(err) => {
            return Err(String::from(err.description()));
        }
    };

    let isa_builder = match *target {
        Some(ref target) => {
            let target = Triple::from_str(&target).map_err(|_| "could not parse --target")?;
            isa::lookup(target).map_err(|err| match err {
                isa::LookupError::SupportDisabled => {
                    "support for architecture disabled at compile time"
                }
                isa::LookupError::Unsupported => "unsupported architecture",
            })?
        }
        None => cranelift_native::builder().unwrap_or_else(|_| {
            panic!("host machine is not a supported target");
        }),
    };
    let flag_builder = settings::builder();
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let mut obj = Artifact::new(isa.triple().clone(), String::from(output));

    // TODO: Expose the tunables as command-line flags.
    let tunables = Tunables::default();

    let (module, lazy_function_body_inputs, lazy_data_initializers, target_config) = {
        let environ = ModuleEnvironment::new(isa.frontend_config(), tunables);

        let translation = environ
            .translate(&data)
            .map_err(|error| error.to_string())?;

        (
            translation.module,
            translation.function_body_inputs,
            translation.data_initializers,
            translation.target_config,
        )
    };

    let (compilation, relocations, address_transform) = cranelift::compile_module(
        &module,
        lazy_function_body_inputs,
        &*isa,
        generate_debug_info,
    )
    .map_err(|e| e.to_string())?;

    emit_module(
        &mut obj,
        &module,
        &compilation,
        &relocations,
        &lazy_data_initializers,
        &target_config,
    )?;

    if generate_debug_info {
        let debug_data = read_debuginfo(&data);
        emit_debugsections(&mut obj, &target_config, &debug_data, &address_transform)
            .map_err(|e| e.to_string())?;
    }

    // FIXME: Make the format a parameter.
    let file =
        ::std::fs::File::create(Path::new(output)).map_err(|x| format(format_args!("{}", x)))?;
    obj.write(file).map_err(|e| e.to_string())?;

    Ok(())
}
