#![deny(trivial_numeric_casts)]
#![warn(unused_import_braces, unstable_features, unused_extern_crates)]
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

use cranelift_codegen::dbg::LOG_FILENAME_PREFIX;
use std::{option::Option, path::PathBuf};
use structopt::StructOpt;

mod bugpoint;
mod cat;
mod compile;
mod disasm;
mod interpret;
mod print_cfg;
mod run;
mod utils;

#[cfg(feature = "souper-harvest")]
mod souper_harvest;

#[cfg(feature = "peepmatic-souper")]
mod souper_to_peepmatic;

#[cfg(feature = "wasm")]
mod wasm;

fn handle_debug_flag(debug: bool) {
    if debug {
        pretty_env_logger::init();
    } else {
        file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
    }
}

/// Cranelift code generator utility.
#[derive(StructOpt)]
enum Commands {
    Test(TestOptions),
    Run(run::Options),
    Interpret(interpret::Options),
    Cat(cat::Options),
    PrintCfg(print_cfg::Options),
    Compile(compile::Options),
    Pass(PassOptions),
    Bugpoint(bugpoint::Options),

    #[cfg(feature = "wasm")]
    Wasm(wasm::Options),
    #[cfg(not(feature = "wasm"))]
    Wasm(CompiledWithoutSupportOptions),

    #[cfg(feature = "peepmatic-souper")]
    SouperToPeepmatic(souper_to_peepmatic::Options),
    #[cfg(not(feature = "peepmatic-souper"))]
    SouperToPeepmatic(CompiledWithoutSupportOptions),

    #[cfg(feature = "souper-harvest")]
    SouperHarvest(souper_harvest::Options),
    #[cfg(not(feature = "souper-harvest"))]
    SouperHarvest(CompiledWithoutSupportOptions),
}

/// Run Cranelift tests
#[derive(StructOpt)]
struct TestOptions {
    /// Be more verbose
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,

    /// Print pass timing report for test
    #[structopt(short = "T")]
    time_passes: bool,

    /// Enable debug output on stderr/stdout
    #[structopt(short = "d")]
    debug: bool,

    /// Specify an input file to be used. Use '-' for stdin.
    #[structopt(required(true), parse(from_os_str))]
    files: Vec<PathBuf>,
}

/// Run specified pass(es) on an input file.
#[derive(StructOpt)]
struct PassOptions {
    /// Be more verbose
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,

    /// Print pass timing report for test
    #[structopt(short = "T")]
    time_passes: bool,

    /// Enable debug output on stderr/stdout
    #[structopt(short = "d")]
    debug: bool,

    /// Specify an input file to be used. Use '-' for stdin.
    #[structopt(parse(from_os_str))]
    file: PathBuf,

    /// Specify the target architecture.
    target: String,

    /// Specify pass(es) to be run on the input file
    #[structopt(required(true))]
    passes: Vec<String>,
}

/// (Compiled without support for this subcommand)
#[derive(StructOpt)]
struct CompiledWithoutSupportOptions {}

fn main() -> anyhow::Result<()> {
    match Commands::from_args() {
        Commands::Cat(c) => cat::run(&c)?,
        Commands::Run(r) => run::run(&r)?,
        Commands::Interpret(i) => interpret::run(&i)?,
        Commands::PrintCfg(p) => print_cfg::run(&p)?,
        Commands::Compile(c) => compile::run(&c)?,
        Commands::Bugpoint(b) => bugpoint::run(&b)?,

        #[cfg(feature = "wasm")]
        Commands::Wasm(w) => wasm::run(&w)?,
        #[cfg(not(feature = "wasm"))]
        Commands::Wasm(_) => anyhow::bail!("Error: clif-util was compiled without wasm support."),

        #[cfg(feature = "peepmatic-souper")]
        Commands::SouperToPeepmatic(s) => souper_to_peepmatic::run(&s)?,
        #[cfg(not(feature = "peepmatic-souper"))]
        Commands::SouperToPeepmatic(_) => anyhow::bail!(
            "Error: clif-util was compiled without support for the `souper-to-peepmatic` \
             subcommand",
        ),

        #[cfg(feature = "souper-harvest")]
        Commands::SouperHarvest(s) => souper_harvest::run(&s)?,
        #[cfg(not(feature = "souper-harvest"))]
        Commands::SouperHarvest(_) => anyhow::bail!(
            "Error: clif-util was compiled without support for the `souper-harvest` \
             subcommand",
        ),

        Commands::Test(t) => {
            handle_debug_flag(t.debug);
            cranelift_filetests::run(
                t.verbose,
                t.time_passes,
                &t.files
                    .iter()
                    .map(|f| f.display().to_string())
                    .collect::<Vec<_>>(),
            )
            .map_err(|s| anyhow::anyhow!("{}", s))?;
        }
        Commands::Pass(p) => {
            handle_debug_flag(p.debug);
            cranelift_filetests::run_passes(
                p.verbose,
                p.time_passes,
                &p.passes,
                &p.target,
                &p.file.display().to_string(),
            )
            .map_err(|s| anyhow::anyhow!("{}", s))?;
        }
    }

    Ok(())
}
