#![deny(trivial_numeric_casts)]
#![warn(unused_import_braces, unstable_features, unused_extern_crates)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::map_unwrap_or,
        clippy::clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

use clap::Parser;
use cranelift_codegen::dbg::LOG_FILENAME_PREFIX;
use std::path::PathBuf;

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
#[derive(Parser)]
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

    #[cfg(feature = "souper-harvest")]
    SouperHarvest(souper_harvest::Options),
    #[cfg(not(feature = "souper-harvest"))]
    SouperHarvest(CompiledWithoutSupportOptions),
}

/// Run Cranelift tests
#[derive(Parser)]
struct TestOptions {
    /// Be more verbose
    #[clap(short, long)]
    verbose: bool,

    /// Print pass timing report for test
    #[clap(short = 'T')]
    time_passes: bool,

    /// Enable debug output on stderr/stdout
    #[clap(short = 'd')]
    debug: bool,

    /// Specify an input file to be used. Use '-' for stdin.
    #[clap(required = true)]
    files: Vec<PathBuf>,
}

/// Run specified pass(es) on an input file.
#[derive(Parser)]
struct PassOptions {
    /// Be more verbose
    #[clap(short, long)]
    verbose: bool,

    /// Print pass timing report for test
    #[clap(short = 'T')]
    time_passes: bool,

    /// Enable debug output on stderr/stdout
    #[clap(short)]
    debug: bool,

    /// Specify an input file to be used. Use '-' for stdin.
    file: PathBuf,

    /// Specify the target architecture.
    target: String,

    /// Specify pass(es) to be run on the input file
    #[clap(required = true)]
    passes: Vec<String>,
}

/// (Compiled without support for this subcommand)
#[derive(Parser)]
struct CompiledWithoutSupportOptions {}

fn main() -> anyhow::Result<()> {
    match Commands::parse() {
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
            )?;
        }
        Commands::Pass(p) => {
            handle_debug_flag(p.debug);
            cranelift_filetests::run_passes(
                p.verbose,
                p.time_passes,
                &p.passes,
                &p.target,
                &p.file.display().to_string(),
            )?;
        }
    }

    Ok(())
}
