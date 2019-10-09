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

use clap::clap_app;
use cranelift_codegen::isa;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_entity::EntityRef;
use cranelift_native;
use cranelift_wasm::DefinedMemoryIndex;
use faerie::Artifact;
use std::env::args_os;
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
use wasmtime::pick_compilation_strategy;
use wasmtime_debug::{emit_debugsections, read_debuginfo};
#[cfg(feature = "lightbeam")]
use wasmtime_environ::Lightbeam;
use wasmtime_environ::{cache_create_new_config, cache_init};
use wasmtime_environ::{
    Compiler, Cranelift, ModuleEnvironment, ModuleVmctxInfo, Tunables, VMOffsets,
};
use wasmtime_jit::CompilationStrategy;
use wasmtime_obj::emit_module;

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let create_cache_config = args_os().any(|arg| arg == "--create-cache-config");
    let matches = if !create_cache_config {
        clap_app!(wasm2obj =>
            (version: version)
            (about: "Wasm to native object translation utility. \
                     Takes a binary WebAssembly module into a native object file. \
                     The translation is dependent on the environment chosen. \
                     The default is a dummy environment that produces placeholder values.")
            (@arg verbose: -v --verbose "displays the module and translated functions")
            (@arg debug: -d --debug "enable debug output on stderr/stdout")
            (@arg debug_info: -g "generate debug information")
            (@arg optimize: -O --optimize "runs optimization passes on the translated functions")
            (@arg enable_simd: --("enable-simd") "enable proposed SIMD instructions")
            (@arg target: --target +takes_value
             "build for the target triple; default is the host machine")
            (@group cache =>
                (@arg disable_cache: --("disable-cache") "disables cache system")
                (@arg cache_config: --("cache-config") +takes_value
                 "use specified cache configuration; can be used with --create-cache-config \
                  to specify custom file")
            )
            (@arg config_path: --("create-cache-config") #{0,1}
             "creates config file; uses default location if none specified")
            (@arg compiler: -C --compiler +takes_value possible_values(&["cranelift", "lightbeam"])
             "choose compiler for all compilation")
            (@arg file: +required +takes_value "input file")
            (@arg output: -o +required +takes_value "output file")
        )
        .get_matches()
    } else {
        clap_app!(wasm2obj =>
            (version: version)
            (about: "Wasm to native object translation utility.\n\n\
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
        let prefix = "wasm2obj.dbg.";
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
                return;
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
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
        process::exit(1);
    }

    let file = matches.value_of_os("file").expect("required argument");
    let path = Path::new(&file);
    match handle_module(
        path.to_path_buf(),
        matches.value_of("target"),
        matches.value_of("output").expect("required argument"),
        matches.is_present("debug_info"),
        matches.is_present("enable_simd"),
        matches.is_present("optimize"),
        matches.value_of("compiler"),
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
    target: Option<&str>,
    output: &str,
    generate_debug_info: bool,
    enable_simd: bool,
    enable_optimize: bool,
    compiler: Option<&str>,
) -> Result<(), String> {
    let data = match read_wasm_file(path) {
        Ok(data) => data,
        Err(err) => {
            return Err(String::from(err.description()));
        }
    };

    let isa_builder = match target {
        Some(target) => {
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
    let strategy = pick_compilation_strategy(compiler);

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
