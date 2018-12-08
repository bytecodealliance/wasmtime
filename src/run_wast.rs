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

extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate docopt;
extern crate wasmtime_wast;
#[macro_use]
extern crate serde_derive;
extern crate file_per_thread_logger;
extern crate pretty_env_logger;

use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use docopt::Docopt;
use std::path::Path;
use wasmtime_wast::WastContext;

static LOG_FILENAME_PREFIX: &str = "cranelift.dbg.";

const USAGE: &str = "
Wast test runner.

Usage:
    run_wast [-do] <file>...
    run_wast --help | --version

Options:
    -h, --help          print this help message
    --version           print the Cranelift version
    -o, --optimize      runs optimization passes on the translated functions
    -d, --debug         enable debug output on stderr/stdout
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    arg_file: Vec<String>,
    flag_debug: bool,
    flag_function: Option<String>,
    flag_optimize: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.help(true)
                .version(Some(String::from("0.0.0")))
                .deserialize()
        })
        .unwrap_or_else(|e| e.exit());
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let mut flag_builder = settings::builder();

    // Enable verifier passes in debug mode.
    if cfg!(debug_assertions) {
        flag_builder.enable("enable_verifier").unwrap();
    }

    if args.flag_debug {
        pretty_env_logger::init();
    } else {
        file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
    }

    // Enable optimization if requested.
    if args.flag_optimize {
        flag_builder.set("opt_level", "best").unwrap();
    }

    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut wast_context = WastContext::new().expect("Error creating WastContext");
    for filename in &args.arg_file {
        wast_context
            .run_file(&*isa, Path::new(&filename))
            .unwrap_or_else(|e| panic!("{}", e));
    }
}
