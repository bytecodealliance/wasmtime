use clap::Parser;
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

    #[cfg(feature = "souper-harvest")]
    SouperHarvest(souper_harvest::Options),
    #[cfg(not(feature = "souper-harvest"))]
    SouperHarvest(CompiledWithoutSupportOptions),
}

/// Run Cranelift tests
#[derive(Parser)]
struct TestOptions {
    /// Be more verbose
    #[arg(short, long)]
    verbose: bool,

    /// Print pass timing report for test
    #[arg(short = 'T')]
    time_passes: bool,

    /// Specify an input file to be used. Use '-' for stdin.
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

/// Run specified pass(es) on an input file.
#[derive(Parser)]
struct PassOptions {
    /// Be more verbose
    #[arg(short, long)]
    verbose: bool,

    /// Print pass timing report for test
    #[arg(short = 'T')]
    time_passes: bool,

    /// Specify an input file to be used. Use '-' for stdin.
    file: PathBuf,

    /// Specify the target architecture.
    target: String,

    /// Specify pass(es) to be run on the input file
    #[arg(required = true)]
    passes: Vec<String>,
}

/// (Compiled without support for this subcommand)
#[derive(Parser)]
struct CompiledWithoutSupportOptions {}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    match Commands::parse() {
        Commands::Cat(c) => cat::run(&c)?,
        Commands::Run(r) => run::run(&r)?,
        Commands::Interpret(i) => interpret::run(&i)?,
        Commands::PrintCfg(p) => print_cfg::run(&p)?,
        Commands::Compile(c) => compile::run(&c)?,
        Commands::Bugpoint(b) => bugpoint::run(&b)?,

        #[cfg(feature = "souper-harvest")]
        Commands::SouperHarvest(s) => souper_harvest::run(&s)?,
        #[cfg(not(feature = "souper-harvest"))]
        Commands::SouperHarvest(_) => anyhow::bail!(
            "Error: clif-util was compiled without support for the `souper-harvest` \
             subcommand",
        ),

        Commands::Test(t) => {
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

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Commands::command().debug_assert()
}
