//! CLI tool to run wast tests using the wasmtime libraries.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
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

use anyhow::{Context, Result};
use docopt::Docopt;
use serde::Deserialize;
use std::path::Path;
use std::process;
use wasmtime::{Config, Engine, HostRef, Store};
use wasmtime_cli::pick_compilation_strategy;
use wasmtime_environ::settings;
use wasmtime_environ::settings::Configurable;
use wasmtime_environ::{cache_create_new_config, cache_init};
use wasmtime_wast::WastContext;

const USAGE: &str = "
Wast test runner.

Usage:
    wast [-do] [--enable-simd] [--disable-cache | --cache-config=<cache_config_file>] [--lightbeam \
                     | --cranelift] <file>...
    wast --create-cache-config [--cache-config=<cache_config_file>]
    wast --help | --version

Options:
    -h, --help          print this help message
    --version           print the Cranelift version
    -o, --optimize      runs optimization passes on the translated functions
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
    -d, --debug         enable debug output on stderr/stdout
    --enable-simd       enable proposed SIMD instructions
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_debug: bool,
    flag_function: Option<String>,
    flag_optimize: bool,
    flag_disable_cache: bool,
    flag_cache_config: Option<String>,
    flag_create_cache_config: bool,
    flag_enable_simd: bool,
    flag_lightbeam: bool,
    flag_cranelift: bool,
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
        let prefix = "cranelift.dbg.";
        wasmtime_cli::init_file_per_thread_logger(prefix);
        Some(prefix)
    };

    if args.flag_create_cache_config {
        let path = cache_create_new_config(args.flag_cache_config)?;
        println!(
            "Successfully created new configuation file at {}",
            path.display()
        );
        return Ok(());
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

    let mut cfg = Config::new();
    let mut flag_builder = settings::builder();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flag_builder.enable("avoid_div_traps").unwrap();

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    // Enable optimization if requested.
    if args.flag_optimize {
        flag_builder.set("opt_level", "speed").unwrap();
    }

    // Enable SIMD if requested
    if args.flag_enable_simd {
        flag_builder.enable("enable_simd").unwrap();
        cfg.wasm_simd(true);
    }

    // Decide how to compile.
    cfg.strategy(pick_compilation_strategy(
        args.flag_cranelift,
        args.flag_lightbeam,
    )?)?
    .flags(settings::Flags::new(flag_builder));
    let store = HostRef::new(Store::new(&Engine::new(&cfg)));
    let mut wast_context = WastContext::new(store);

    wast_context
        .register_spectest()
        .context("error instantiating \"spectest\"")?;

    for filename in &args.arg_file {
        wast_context.run_file(Path::new(&filename))?;
    }
    Ok(())
}
