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

use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{isa, settings};
use cranelift_entity::EntityRef;
use cranelift_wasm::DefinedMemoryIndex;
use docopt::Docopt;
use faerie::Artifact;
use serde::Deserialize;
use std::error::Error;
use std::fmt::format;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{process, str};
use target_lexicon::Triple;
use wasmtime_cli::pick_compilation_strategy;
use wasmtime_debug::{emit_debugsections, read_debuginfo};
#[cfg(feature = "lightbeam")]
use wasmtime_environ::Lightbeam;
use wasmtime_environ::{
    cache_create_new_config, cache_init, Compiler, Cranelift, ModuleEnvironment, ModuleVmctxInfo,
    Tunables, VMOffsets,
};
use wasmtime_jit::CompilationStrategy;
use wasmtime_obj::emit_module;

const USAGE: &str = "
Wasm to native object translation utility.
Takes a binary WebAssembly module into a native object file.
The translation is dependent on the environment chosen.
The default is a dummy environment that produces placeholder values.

Usage:
    wasm2obj [--target TARGET] [-Odg] [--disable-cache | --cache-config=<cache_config_file>] \
                     [--enable-simd] [--lightbeam | --cranelift] <file> -o <output>
    wasm2obj --create-cache-config [--cache-config=<cache_config_file>]
    wasm2obj --help | --version

Options:
    -v, --verbose       displays the module and translated functions
    -h, --help          print this help message
    --target <TARGET>   build for the target triple; default is the host machine
    -g                  generate debug information
    --disable-cache     disables cache system
    --cache-config=<cache_config_file>
                        use specified cache configuration;
                        can be used with --create-cache-config to specify custom file
    --create-cache-config
                        creates default configuration and writes it to the disk,
                        use with --cache-config to specify custom config file
                        instead of default one
    --lightbeam         use Lightbeam for all compilation
    --cranelift         use Cranelift for all compilation
    --enable-simd       enable proposed SIMD instructions
    -O, --optimize      runs optimization passes on the translated functions
    --version           print the Cranelift version
    -d, --debug         enable debug output on stderr/stdout
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: String,
    arg_output: String,
    arg_target: Option<String>,
    flag_g: bool,
    flag_debug: bool,
    flag_disable_cache: bool,
    flag_cache_config: Option<String>,
    flag_create_cache_config: bool,
    flag_enable_simd: bool,
    flag_lightbeam: bool,
    flag_cranelift: bool,
    flag_optimize: bool,
}

fn main() {
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
        let prefix = "wasm2obj.dbg.";
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
                return;
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
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
        process::exit(1);
    }

    let path = Path::new(&args.arg_file);
    match handle_module(
        path.to_path_buf(),
        &args.arg_target,
        &args.arg_output,
        args.flag_g,
        args.flag_enable_simd,
        args.flag_optimize,
        args.flag_cranelift,
        args.flag_lightbeam,
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
    enable_simd: bool,
    enable_optimize: bool,
    cranelift: bool,
    lightbeam: bool,
) -> Result<(), String> {
    let data = match wat::parse_file(path) {
        Ok(data) => data,
        Err(err) => {
            return Err(String::from(err.description()));
        }
    };

    let isa_builder = match *target {
        Some(ref target) => {
            let target = Triple::from_str(&target).map_err(|_| "could not parse --target")?;
            isa::lookup(target).map_err(|err| format!("{:?}", err))?
        }
        None => cranelift_native::builder().unwrap_or_else(|_| {
            panic!("host machine is not a supported target");
        }),
    };
    let mut flag_builder = settings::builder();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flag_builder.enable("avoid_div_traps").unwrap();

    if enable_simd {
        flag_builder.enable("enable_simd").unwrap();
    }

    if enable_optimize {
        flag_builder.set("opt_level", "speed").unwrap();
    }

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let mut obj = Artifact::new(isa.triple().clone(), String::from(output));

    // TODO: Expose the tunables as command-line flags.
    let tunables = Tunables::default();

    // Decide how to compile.
    let strategy = pick_compilation_strategy(cranelift, lightbeam);

    let (
        module,
        module_translation,
        lazy_function_body_inputs,
        lazy_data_initializers,
        target_config,
    ) = {
        let environ = ModuleEnvironment::new(isa.frontend_config(), tunables);

        let translation = environ
            .translate(&data)
            .map_err(|error| error.to_string())?;

        (
            translation.module,
            translation.module_translation.unwrap(),
            translation.function_body_inputs,
            translation.data_initializers,
            translation.target_config,
        )
    };

    // TODO: use the traps information
    let (compilation, relocations, address_transform, value_ranges, stack_slots, _traps) =
        match strategy {
            CompilationStrategy::Auto | CompilationStrategy::Cranelift => {
                Cranelift::compile_module(
                    &module,
                    &module_translation,
                    lazy_function_body_inputs,
                    &*isa,
                    generate_debug_info,
                )
                .map_err(|e| e.to_string())?
            }
            #[cfg(feature = "lightbeam")]
            CompilationStrategy::Lightbeam => Lightbeam::compile_module(
                &module,
                &module_translation,
                lazy_function_body_inputs,
                &*isa,
                generate_debug_info,
            )
            .map_err(|e| e.to_string())?,
        };

    let module_vmctx_info = {
        let ofs = VMOffsets::new(target_config.pointer_bytes(), &module);
        let memory_offset = ofs.vmctx_vmmemory_definition_base(DefinedMemoryIndex::new(0)) as i64;
        ModuleVmctxInfo {
            memory_offset,
            stack_slots,
        }
    };

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
        emit_debugsections(
            &mut obj,
            &module_vmctx_info,
            &target_config,
            &debug_data,
            &address_transform,
            &value_ranges,
        )
        .map_err(|e| e.to_string())?;
    }

    // FIXME: Make the format a parameter.
    let file =
        ::std::fs::File::create(Path::new(output)).map_err(|x| format(format_args!("{}", x)))?;
    obj.write(file).map_err(|e| e.to_string())?;

    Ok(())
}
