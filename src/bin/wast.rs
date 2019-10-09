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

use clap::clap_app;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_native;
use pretty_env_logger;
use std::path::Path;
use std::process;
use wasmtime::pick_compilation_strategy;
use wasmtime_environ::cache_init;
use wasmtime_jit::{Compiler, Features};
use wasmtime_wast::WastContext;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let matches = clap_app!(wast =>
        (version: version)
        (about: "Wast test runner")
        (@arg debug: -d --debug "enable debug output on stderr/stdout")
        (@arg optimize: -O --optimize "runs optimization passes on the translated functions")
        (@arg enable_simd: --("enable-simd") "enable proposed SIMD instructions")
        (@group cache =>
            (@arg disable_cache: --("disable-cache") "disables cache system")
            (@arg cache_config: --("cache-config") +takes_value
            "use specified cache configuration; can be used with --create-cache-config \
             to specify custom file")
        )
        (@arg compiler: -C --compiler +takes_value possible_values(&["cranelift", "lightbeam"])
         "choose compiler for all compilation")
        (@arg file: +required +takes_value "input file")
    )
    .get_matches();

    let log_config = if matches.is_present("debug") {
        pretty_env_logger::init();
        None
    } else {
        let prefix = "cranelift.dbg.";
        wasmtime::init_file_per_thread_logger(prefix);
        Some(prefix)
    };

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

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let mut flag_builder = settings::builder();
    let mut features: Features = Default::default();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flag_builder.enable("avoid_div_traps").unwrap();

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    // Enable optimization if requested.
    if matches.is_present("optimize") {
        flag_builder.set("opt_level", "speed").unwrap();
    }

    // Enable SIMD if requested
    if matches.is_present("enable_simd") {
        flag_builder.enable("enable_simd").unwrap();
        features.simd = true;
    }

    // Decide how to compile.
    let strategy = pick_compilation_strategy(matches.value_of("compiler"));

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let engine = Compiler::new(isa, strategy);
    let mut wast_context = WastContext::new(Box::new(engine)).with_features(features);

    wast_context
        .register_spectest()
        .expect("error instantiating \"spectest\"");

    for filename in &matches.value_of("file") {
        wast_context
            .run_file(Path::new(&filename))
            .unwrap_or_else(|e| {
                eprintln!("{}", e);
                process::exit(1)
            });
    }
}
