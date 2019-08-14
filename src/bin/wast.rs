//! CLI tool to run wast tests using the wasmtime libraries.

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

use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_native;
use docopt::Docopt;
use pretty_env_logger;
use serde::Deserialize;
use std::path::Path;
use std::process;
use wasmtime_environ::cache_conf;
use wasmtime_jit::{Compiler, Features};
use wasmtime_wast::WastContext;

const USAGE: &str = "
Wast test runner.

Usage:
    wast [-do] [--enable-simd] [--cache | --cache-dir=<cache_dir>] <file>...
    wast --help | --version

Options:
    -h, --help          print this help message
    --version           print the Cranelift version
    -o, --optimize      runs optimization passes on the translated functions
    -c, --cache         enable caching system, use default cache directory
    --cache-dir=<cache_dir>
                        enable caching system, use specified cache directory
    -d, --debug         enable debug output on stderr/stdout
    --enable-simd       enable proposed SIMD instructions
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_debug: bool,
    flag_function: Option<String>,
    flag_optimize: bool,
    flag_cache: bool,
    flag_cache_dir: Option<String>,
    flag_enable_simd: bool,
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

    if args.flag_debug {
        pretty_env_logger::init();
    } else {
        wasmtime::init_file_per_thread_logger("cranelift.dbg.");
    }

    cache_conf::init(
        args.flag_cache || args.flag_cache_dir.is_some(),
        args.flag_cache_dir.as_ref(),
    );

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let mut flag_builder = settings::builder();
    let mut features: Features = Default::default();

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    // Enable optimization if requested.
    if args.flag_optimize {
        flag_builder.set("opt_level", "best").unwrap();
    }

    // Enable SIMD if requested
    if args.flag_enable_simd {
        flag_builder.enable("enable_simd").unwrap();
        features.simd = true;
    }

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let engine = Compiler::new(isa);
    let mut wast_context = WastContext::new(Box::new(engine)).with_features(features);

    wast_context
        .register_spectest()
        .expect("error instantiating \"spectest\"");

    for filename in &args.arg_file {
        wast_context
            .run_file(Path::new(&filename))
            .unwrap_or_else(|e| {
                eprintln!("{}", e);
                process::exit(1)
            });
    }
}
