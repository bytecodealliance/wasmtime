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

use clap::{clap_app, ArgMatches, Values};
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use failure::{bail, Error, ResultExt};
use pretty_env_logger;
use std::collections::HashMap;
use std::env::args_os;
use std::ffi::OsStr;
use std::fs::File;
use std::iter::Iterator;
use std::path::Component;
use std::path::Path;
use std::process::exit;
use wasi_common::preopen_dir;
use wasmtime::pick_compilation_strategy;
use wasmtime_api::{Config, Engine, HostRef, Instance, Module, Store};
use wasmtime_environ::{cache_create_new_config, cache_init};
use wasmtime_interface_types::ModuleData;
use wasmtime_jit::Features;
use wasmtime_wasi::instantiate_wasi;
use wasmtime_wast::instantiate_spectest;

#[cfg(feature = "wasi-c")]
use wasmtime_wasi_c::instantiate_wasi_c;

fn compute_preopen_dirs<'a>(
    flag_dir: impl Iterator<Item = &'a str>,
    flag_mapdir: impl Iterator<Item = &'a str>,
) -> Vec<(String, File)> {
    let mut preopen_dirs = Vec::new();

    for dir in flag_dir {
        let preopen_dir = preopen_dir(dir).unwrap_or_else(|err| {
            println!("error while pre-opening directory {}: {}", dir, err);
            exit(1);
        });
        preopen_dirs.push((dir.to_string(), preopen_dir));
    }

    for mapdir in flag_mapdir {
        let parts: Vec<&str> = mapdir.split("::").collect();
        if parts.len() != 2 {
            println!("--mapdir argument must contain exactly one double colon ('::'), separating a guest directory name and a host directory name");
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
fn compute_argv<'a>(argv0: &str, arg_arg: impl Iterator<Item = &'a str>) -> Vec<String> {
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
fn compute_environ<'a>(flag_env: impl Iterator<Item = &'a str>) -> Vec<(String, String)> {
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
    let err = match rmain() {
        Ok(()) => return,
        Err(e) => e,
    };
    eprintln!("error: {}", err);
    for cause in err.iter_causes() {
        eprintln!("    caused by: {}", cause);
    }
    exit(1);
}

fn rmain() -> Result<(), Error> {
    let version = env!("CARGO_PKG_VERSION");
    let create_cache_config = args_os().any(|arg| arg == "--create-cache-config");
    let matches = if !create_cache_config {
        clap_app!(wasmtime =>
            (version: version)
            (about: "Wasm runner.\n\n\
                     Takes a binary (wasm) or text (wat) WebAssembly module and instantiates it, \
                     including calling the start function if one is present. Additional functions \
                     given with --invoke are then called.")
            (@arg debug: -d --debug "enable debug output on stderr/stdout")
            (@arg debug_info: -g "generate debug information")
            (@arg optimize: -O --optimize "runs optimization passes on the translated functions")
            (@arg enable_simd: --("enable-simd") "enable proposed SIMD instructions")
            (@arg wasi_c: --("wasi-c") "enable the wasi-c implementation of WASI")
            (@group cache =>
                (@arg disable_cache: --("disable-cache") "disables cache system")
                (@arg cache_config: --("cache-config") +takes_value
                 "use specified cache configuration; can be used with --create-cache-config \
                  to specify custom file")
            )
            (@arg config_path: --("create-cache-config") #{0,1}
             "creates config file; uses default location if none specified")
            (@arg env: --env ... number_of_values(1)
             "pass an environment variable (\"key=value\") to the program")
            (@arg dir: --dir ... number_of_values(1) "grant access to the given host directory")
            (@arg mapdir: --mapdir ... number_of_values(1)
             "where <mapping> has the form <wasmdir>::<hostdir>, grant access to the given host \
              directory with the given wasm directory name")
            (@arg compiler: -C --compiler +takes_value possible_values(&["cranelift", "lightbeam"])
             "choose compiler for all compilation")
            (@group action =>
                (@arg fn: --invoke +takes_value "name of function to run")
                (@arg wasm: --preload ... number_of_values(1)
                 "load an additional wasm module before loading the main module")
            )
            (@arg file: +required +takes_value "input file")
            (@arg arg: ... "argument")
        )
        .get_matches()
    } else {
        clap_app!(wasmtime =>
            (version: version)
            (about: "Wasm runner.\n\n\
                     Below you can find options compatible with --create-cache-config.")
            (@arg debug: -d --debug "enable debug output on stderr/stdout")
            (@arg config_path: --("create-cache-config") #{0,1}
             "creates config file; uses default location if none specified")
        )
        .get_matches()
    };

    let log_config = if matches.is_present("debug") {
        pretty_env_logger::init();
        None
    } else {
        let prefix = "wasmtime.dbg.";
        wasmtime::init_file_per_thread_logger(prefix);
        Some(prefix)
    };

    if matches.is_present("config_path") {
        match cache_create_new_config(matches.value_of("config_path")) {
            Ok(path) => {
                println!(
                    "Successfully created new configuation file at {}",
                    path.display()
                );
                return Ok(());
            }
            // String doesn't implement Error
            Err(err) => {
                eprintln!("Error: {}", err);
                exit(1);
            }
        }
    }

    let errors = cache_init(
        !matches.is_present("disable_cache"),
        matches.value_of("cache_config"),
        log_config,
    );

    if !errors.is_empty() {
        eprintln!("Cache initialization failed. Errors:");
        for e in errors {
            eprintln!("-> {}", e);
        }
        exit(1);
    }

    let mut flag_builder = settings::builder();
    let mut features: Features = Default::default();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flag_builder.enable("avoid_div_traps")?;

    // Enable/disable producing of debug info.
    let debug_info = matches.is_present("debug_info");

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier")?;
    }

    // Enable SIMD if requested
    if matches.is_present("enable_simd") {
        flag_builder.enable("enable_simd")?;
        features.simd = true;
    }

    // Enable optimization if requested.
    if matches.is_present("optimize") {
        flag_builder.set("opt_level", "speed")?;
    }

    // Decide how to compile.
    let strategy = pick_compilation_strategy(matches.value_of("compiler"));

    let config = Config::new(
        settings::Flags::new(flag_builder),
        features,
        debug_info,
        strategy,
    );
    let engine = HostRef::new(Engine::new(config));
    let store = HostRef::new(Store::new(engine));

    let mut module_registry = HashMap::new();

    // Make spectest available by default.
    module_registry.insert(
        "spectest".to_owned(),
        Instance::from_handle(store.clone(), instantiate_spectest()?)?,
    );

    // Make wasi available by default.
    let file = &matches.value_of("file").expect("required argument");
    let global_exports = store.borrow().global_exports().clone();
    let preopen_dirs = compute_preopen_dirs(
        matches.values_of("dir").unwrap_or_else(Values::default),
        matches.values_of("mapdir").unwrap_or_else(Values::default),
    );
    let argv = compute_argv(
        file,
        matches.values_of("arg").unwrap_or_else(Values::default),
    );
    let environ = compute_environ(matches.values_of("env").unwrap_or_else(Values::default));

    let wasi = if matches.is_present("wasi_c") {
        #[cfg(feature = "wasi-c")]
        {
            instantiate_wasi_c("", global_exports.clone(), &preopen_dirs, &argv, &environ)?
        }
        #[cfg(not(feature = "wasi-c"))]
        {
            bail!("wasi-c feature not enabled at build time")
        }
    } else {
        instantiate_wasi("", global_exports.clone(), &preopen_dirs, &argv, &environ)?
    };

    module_registry.insert(
        "wasi_unstable".to_owned(),
        Instance::from_handle(store.clone(), wasi.clone())?,
    );
    module_registry.insert(
        "wasi_unstable_preview0".to_owned(),
        Instance::from_handle(store.clone(), wasi)?,
    );

    // Load the preload wasm modules.
    for filename in matches.values_of("wasm").unwrap_or_else(Values::default) {
        let path = Path::new(&filename);
        instantiate_module(store.clone(), &module_registry, path)
            .with_context(|_| format!("failed to process preload at `{}`", path.display()))?;
    }

    // Load the main wasm module.
    let path = Path::new(&file);
    handle_module(store, &module_registry, &matches, path)
        .with_context(|_| format!("failed to process main module `{}`", path.display()))?;
    Ok(())
}

fn instantiate_module(
    store: HostRef<Store>,
    module_registry: &HashMap<String, (Instance, HashMap<String, usize>)>,
    path: &Path,
) -> Result<(HostRef<Instance>, HostRef<Module>, Vec<u8>), Error> {
    // Read the wasm module binary either as `*.wat` or a raw binary
    let data = wat::parse_file(path.to_path_buf())?;

    let module = HostRef::new(Module::new(store.clone(), &data)?);

    // Resolve import using module_registry.
    let imports = module
        .borrow()
        .imports()
        .iter()
        .map(|i| {
            let module_name = i.module().to_string();
            if let Some((instance, map)) = module_registry.get(&module_name) {
                let field_name = i.name().to_string();
                if let Some(export_index) = map.get(&field_name) {
                    Ok(instance.exports()[*export_index].clone())
                } else {
                    bail!(
                        "Import {} was not found in module {}",
                        field_name,
                        module_name
                    )
                }
            } else {
                bail!("Import module {} was not found", module_name)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let instance = HostRef::new(Instance::new(store.clone(), module.clone(), &imports)?);

    Ok((instance, module, data))
}

fn handle_module(
    store: HostRef<Store>,
    module_registry: &HashMap<String, (Instance, HashMap<String, usize>)>,
    matches: &ArgMatches,
    path: &Path,
) -> Result<(), Error> {
    let (instance, _module, data) = instantiate_module(store.clone(), module_registry, path)?;

    // If a function to invoke was given, invoke it.
    if let Some(f) = &matches.value_of("fn") {
        let data = ModuleData::new(&data)?;
        invoke_export(store, instance, &data, f, matches)?;
    }

    Ok(())
}

fn invoke_export(
    store: HostRef<Store>,
    instance: HostRef<Instance>,
    data: &ModuleData,
    name: &str,
    matches: &ArgMatches,
) -> Result<(), Error> {
    use wasm_webidl_bindings::ast;
    use wasmtime_interface_types::Value;

    let mut handle = instance.borrow().handle().clone();

    // Use the binding information in `ModuleData` to figure out what arguments
    // need to be passed to the function that we're invoking. Currently we take
    // the CLI parameters and attempt to parse them into function arguments for
    // the function we'll invoke.
    let binding = data.binding_for_export(&mut handle, name)?;
    if binding.param_types()?.len() > 0 {
        eprintln!(
            "warning: using `--render` with a function that takes arguments \
             is experimental and may break in the future"
        );
    }
    let mut values = Vec::new();
    let mut args = matches.values_of("arg").unwrap_or_else(Values::default);
    for ty in binding.param_types()? {
        let val = match args.next() {
            Some(s) => s,
            None => bail!("not enough arguments for `{}`", name),
        };
        values.push(match ty {
            // TODO: integer parsing here should handle hexadecimal notation
            // like `0x0...`, but the Rust standard library currently only
            // parses base-10 representations.
            ast::WebidlScalarType::Long => Value::I32(val.parse()?),
            ast::WebidlScalarType::LongLong => Value::I64(val.parse()?),
            ast::WebidlScalarType::UnsignedLong => Value::U32(val.parse()?),
            ast::WebidlScalarType::UnsignedLongLong => Value::U64(val.parse()?),

            ast::WebidlScalarType::Float | ast::WebidlScalarType::UnrestrictedFloat => {
                Value::F32(val.parse()?)
            }
            ast::WebidlScalarType::Double | ast::WebidlScalarType::UnrestrictedDouble => {
                Value::F64(val.parse()?)
            }
            ast::WebidlScalarType::DomString => Value::String(val.to_string()),
            t => bail!("unsupported argument type {:?}", t),
        });
    }

    // Invoke the function and then afterwards print all the results that came
    // out, if there are any.
    let mut context = store.borrow().engine().borrow().create_wasmtime_context();
    let results = data
        .invoke(&mut context, &mut handle, name, &values)
        .with_context(|_| format!("failed to invoke `{}`", name))?;
    if results.len() > 0 {
        eprintln!(
            "warning: using `--render` with a function that returns values \
             is experimental and may break in the future"
        );
    }
    for result in results {
        println!("{}", result);
    }

    Ok(())
}
