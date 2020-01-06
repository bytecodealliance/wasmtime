//! CLI tool to use the functions provided by the [wasmtime](../wasmtime/index.html)
//! crate.
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cranelift
//! IL. Can also execute the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
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

use anyhow::{bail, Context as _, Result};
use docopt::Docopt;
use serde::Deserialize;
use std::path::{Component, Path};
use std::{collections::HashMap, ffi::OsStr, fs::File, process::exit};
use wasi_common::preopen_dir;
use wasmtime::{Config, Engine, HostRef, Instance, Module, Store};
use wasmtime_cli::pick_compilation_strategy;
use wasmtime_environ::{cache_create_new_config, cache_init};
use wasmtime_environ::{settings, settings::Configurable};
use wasmtime_interface_types::ModuleData;
use wasmtime_jit::Features;
use wasmtime_wasi::create_wasi_instance;
use wasmtime_wasi::old::snapshot_0::create_wasi_instance as create_wasi_instance_snapshot_0;
#[cfg(feature = "wasi-c")]
use wasmtime_wasi_c::instantiate_wasi_c;

const USAGE: &str = "
Wasm runner.

Takes a binary (wasm) or text (wat) WebAssembly module and instantiates it,
including calling the start function if one is present. Additional functions
given with --invoke are then called.

Usage:
    wasmtime [-odg] [--enable-simd] [--wasi-c] [--disable-cache | \
     --cache-config=<cache_config_file>] [--preload=<wasm>...] [--env=<env>...] [--dir=<dir>...] \
     [--mapdir=<mapping>...] [--lightbeam | --cranelift] <file> [<arg>...]
    wasmtime [-odg] [--enable-simd] [--wasi-c] [--disable-cache | \
     --cache-config=<cache_config_file>] [--env=<env>...] [--dir=<dir>...] \
     [--mapdir=<mapping>...] --invoke=<fn> [--lightbeam | --cranelift] <file> [<arg>...]
    wasmtime --create-cache-config [--cache-config=<cache_config_file>]
    wasmtime --help | --version

Options:
    --invoke=<fn>       name of function to run
    -o, --optimize      runs optimization passes on the translated functions
    --disable-cache     disables cache system
    --cache-config=<cache_config_file>
                        use specified cache configuration;
                        can be used with --create-cache-config to specify custom file
    --create-cache-config
                        creates default configuration and writes it to the disk,
                        use with --cache-config to specify custom config file
                        instead of default one
    -g                  generate debug information
    -d, --debug         enable debug output on stderr/stdout
    --lightbeam         use Lightbeam for all compilation
    --cranelift         use Cranelift for all compilation
    --enable-simd       enable proposed SIMD instructions
    --wasi-c            enable the wasi-c implementation of `wasi_unstable`
    --preload=<wasm>    load an additional wasm module before loading the main module
    --env=<env>         pass an environment variable (\"key=value\") to the program
    --dir=<dir>         grant access to the given host directory
    --mapdir=<mapping>  where <mapping> has the form <wasmdir>::<hostdir>, grant access to
                        the given host directory with the given wasm directory name
    -h, --help          print this help message
    --version           print the Cranelift version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: String,
    arg_arg: Vec<String>,
    flag_optimize: bool,
    flag_disable_cache: bool,
    flag_cache_config: Option<String>,
    flag_create_cache_config: bool,
    flag_debug: bool,
    flag_g: bool,
    flag_enable_simd: bool,
    flag_lightbeam: bool,
    flag_cranelift: bool,
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
        let parts: Vec<&str> = mapdir.split("::").collect();
        if parts.len() != 2 {
            println!(
                "--mapdir argument must contain exactly one double colon ('::'), separating a \
                 guest directory name and a host directory name"
            );
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

fn main() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.help(true)
                .version(Some(String::from(version)))
                .deserialize()
        })
        .unwrap_or_else(|e| e.exit());

    let log_config = if args.flag_debug {
        pretty_env_logger::init();
        None
    } else {
        let prefix = "wasmtime.dbg.";
        wasmtime_cli::init_file_per_thread_logger(prefix);
        Some(prefix)
    };

    if args.flag_create_cache_config {
        match cache_create_new_config(args.flag_cache_config) {
            Ok(path) => {
                println!(
                    "Successfully created new configuation file at {}",
                    path.display()
                );
                return Ok(());
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                exit(1);
            }
        }
    }

    let errors = cache_init(
        !args.flag_disable_cache,
        args.flag_cache_config.as_ref(),
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
    let debug_info = args.flag_g;

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier")?;
    }

    // Enable SIMD if requested
    if args.flag_enable_simd {
        flag_builder.enable("enable_simd")?;
        features.simd = true;
    }

    // Enable optimization if requested.
    if args.flag_optimize {
        flag_builder.set("opt_level", "speed")?;
    }

    // Decide how to compile.
    let strategy = pick_compilation_strategy(args.flag_cranelift, args.flag_lightbeam);

    let mut config = Config::new();
    config
        .features(features)
        .flags(settings::Flags::new(flag_builder))
        .debug_info(debug_info)
        .strategy(strategy);
    let engine = Engine::new(&config);
    let store = HostRef::new(Store::new(&engine));

    let mut module_registry = HashMap::new();

    // Make wasi available by default.
    let preopen_dirs = compute_preopen_dirs(&args.flag_dir, &args.flag_mapdir);
    let argv = compute_argv(&args.arg_file, &args.arg_arg);
    let environ = compute_environ(&args.flag_env);

    let wasi_unstable = HostRef::new(if args.flag_wasi_c {
        #[cfg(feature = "wasi-c")]
        {
            let global_exports = store.borrow().global_exports().clone();
            let handle = instantiate_wasi_c(global_exports, &preopen_dirs, &argv, &environ)?;
            Instance::from_handle(&store, handle)
        }
        #[cfg(not(feature = "wasi-c"))]
        {
            bail!("wasi-c feature not enabled at build time")
        }
    } else {
        create_wasi_instance_snapshot_0(&store, &preopen_dirs, &argv, &environ)?
    });

    let wasi_snapshot_preview1 = HostRef::new(create_wasi_instance(
        &store,
        &preopen_dirs,
        &argv,
        &environ,
    )?);

    module_registry.insert("wasi_unstable".to_owned(), wasi_unstable);
    module_registry.insert("wasi_snapshot_preview1".to_owned(), wasi_snapshot_preview1);

    // Load the preload wasm modules.
    for filename in &args.flag_preload {
        let path = Path::new(&filename);
        instantiate_module(&store, &module_registry, path)
            .with_context(|| format!("failed to process preload at `{}`", path.display()))?;
    }

    // Load the main wasm module.
    let path = Path::new(&args.arg_file);
    handle_module(&store, &module_registry, &args, path)
        .with_context(|| format!("failed to process main module `{}`", path.display()))?;
    Ok(())
}

fn instantiate_module(
    store: &HostRef<Store>,
    module_registry: &HashMap<String, HostRef<Instance>>,
    path: &Path,
) -> Result<(HostRef<Instance>, HostRef<Module>, Vec<u8>)> {
    // Read the wasm module binary either as `*.wat` or a raw binary
    let data = wat::parse_file(path.to_path_buf())?;

    let module = HostRef::new(Module::new(store, &data)?);

    // Resolve import using module_registry.
    let imports = module
        .borrow()
        .imports()
        .iter()
        .map(|i| {
            let module_name = i.module();
            if let Some(instance) = module_registry.get(module_name) {
                let field_name = i.name();
                if let Some(export) = instance.borrow().find_export_by_name(field_name) {
                    Ok(export.clone())
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

    let instance = HostRef::new(match Instance::new(store, &module, &imports) {
        Ok(instance) => instance,
        Err(trap) => bail!("Failed to instantiate {:?}: {:?}", path, trap),
    });

    Ok((instance, module, data))
}

fn handle_module(
    store: &HostRef<Store>,
    module_registry: &HashMap<String, HostRef<Instance>>,
    args: &Args,
    path: &Path,
) -> Result<()> {
    let (instance, module, data) = instantiate_module(store, module_registry, path)?;

    // If a function to invoke was given, invoke it.
    if let Some(f) = &args.flag_invoke {
        let data = ModuleData::new(&data)?;
        invoke_export(instance, &data, f, args)?;
    } else if module
        .borrow()
        .exports()
        .iter()
        .any(|export| export.name().is_empty())
    {
        // Launch the default command export.
        let data = ModuleData::new(&data)?;
        invoke_export(instance, &data, "", args)?;
    } else {
        // If the module doesn't have a default command export, launch the
        // _start function if one is present, as a compatibility measure.
        let data = ModuleData::new(&data)?;
        invoke_export(instance, &data, "_start", args)?;
    }

    Ok(())
}

fn invoke_export(
    instance: HostRef<Instance>,
    data: &ModuleData,
    name: &str,
    args: &Args,
) -> Result<()> {
    use wasm_webidl_bindings::ast;
    use wasmtime_interface_types::Value;

    let mut handle = instance.borrow().handle().clone();

    // Use the binding information in `ModuleData` to figure out what arguments
    // need to be passed to the function that we're invoking. Currently we take
    // the CLI parameters and attempt to parse them into function arguments for
    // the function we'll invoke.
    let binding = data.binding_for_export(&mut handle, name)?;
    if !binding.param_types()?.is_empty() {
        eprintln!(
            "warning: using `--invoke` with a function that takes arguments \
             is experimental and may break in the future"
        );
    }
    let mut values = Vec::new();
    let mut args = args.arg_arg.iter();
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
    let results = data
        .invoke_export(&instance, name, &values)
        .with_context(|| format!("failed to invoke `{}`", name))?;
    if !results.is_empty() {
        eprintln!(
            "warning: using `--invoke` with a function that returns values \
             is experimental and may break in the future"
        );
    }
    for result in results {
        println!("{}", result);
    }

    Ok(())
}
