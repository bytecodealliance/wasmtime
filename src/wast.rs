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
use file_per_thread_logger;
use pretty_env_logger;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;
use wasmtime_jit::Compiler;
use wasmtime_wast::WastContext;

static LOG_FILENAME_PREFIX: &str = "cranelift.dbg.";

/// Wast test runner.
#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "wast")]
struct Args {
    #[structopt(name = "FILE", parse(from_os_str))]
    arg_file: Vec<PathBuf>,

    /// enable debug output on stderr/stdout
    #[structopt(short = "d", long = "debug")]
    flag_debug: bool,

    #[structopt(long = "function")]
    flag_function: Option<String>,

    /// runs optimization passes on the translated functions
    #[structopt(short = "o", long = "optimize")]
    flag_optimize: bool,
}

fn main() {
    let args = Args::from_args();

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
    let engine = Compiler::new(isa);
    let mut wast_context = WastContext::new(Box::new(engine));

    wast_context
        .register_spectest()
        .expect("error instantiating \"spectest\"");

    for filename in &args.arg_file {
        wast_context.run_file(&filename).unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1)
        });
    }
}
