//! The `wasmtime` command line tool.
//!
//! Primarily used to run WebAssembly modules.
//! See `wasmtime --help` for usage.

use anyhow::Result;
use clap::Parser;

/// Wasmtime WebAssembly Runtime
#[derive(Parser)]
#[command(
    name = "wasmtime",
    version = version(),
    after_help = "If a subcommand is not provided, the `run` subcommand will be used.\n\
                  \n\
                  Usage examples:\n\
                  \n\
                  Running a WebAssembly module with a start function:\n\
                  \n  \
                  wasmtime example.wasm
                  \n\
                  Passing command line arguments to a WebAssembly module:\n\
                  \n  \
                  wasmtime example.wasm arg1 arg2 arg3\n\
                  \n\
                  Invoking a specific function (e.g. `add`) in a WebAssembly module:\n\
                  \n  \
                  wasmtime --invoke add example.wasm 1 2\n",

    // This option enables the pattern below where we ask clap to parse twice
    // sorta: once where it's trying to find a subcommand and once assuming
    // a subcommand doesn't get passed. Clap should then, apparently,
    // fill in the `subcommand` if found and otherwise fill in the
    // `RunCommand`.
    args_conflicts_with_subcommands = true
)]
struct Wasmtime {
    #[cfg(not(feature = "run"))]
    #[command(subcommand)]
    subcommand: Subcommand,

    #[cfg(feature = "run")]
    #[command(subcommand)]
    subcommand: Option<Subcommand>,
    #[command(flatten)]
    #[cfg(feature = "run")]
    run: wasmtime_cli::commands::RunCommand,
}

/// If WASMTIME_VERSION_INFO is set, use it, otherwise use CARGO_PKG_VERSION.
fn version() -> &'static str {
    option_env!("WASMTIME_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Parser)]
enum Subcommand {
    /// Runs a WebAssembly module
    #[cfg(feature = "run")]
    Run(wasmtime_cli::commands::RunCommand),

    /// Controls Wasmtime configuration settings
    #[cfg(feature = "cache")]
    Config(wasmtime_cli::commands::ConfigCommand),

    /// Compiles a WebAssembly module.
    #[cfg(feature = "compile")]
    Compile(wasmtime_cli::commands::CompileCommand),

    /// Explore the compilation of a WebAssembly module to native code.
    #[cfg(feature = "explore")]
    Explore(wasmtime_cli::commands::ExploreCommand),

    /// Serves requests from a wasi-http proxy component.
    #[cfg(feature = "serve")]
    Serve(wasmtime_cli::commands::ServeCommand),

    /// Displays available Cranelift settings for a target.
    #[cfg(feature = "cranelift")]
    Settings(wasmtime_cli::commands::SettingsCommand),

    /// Runs a WebAssembly test script file
    #[cfg(feature = "wast")]
    Wast(wasmtime_cli::commands::WastCommand),

    /// Generate shell completions for the `wasmtime` CLI
    #[cfg(feature = "completion")]
    Completion(CompletionCommand),
}

impl Wasmtime {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        #[cfg(feature = "run")]
        let subcommand = self.subcommand.unwrap_or(Subcommand::Run(self.run));
        #[cfg(not(feature = "run"))]
        let subcommand = self.subcommand;

        match subcommand {
            #[cfg(feature = "run")]
            Subcommand::Run(c) => c.execute(),

            #[cfg(feature = "cache")]
            Subcommand::Config(c) => c.execute(),

            #[cfg(feature = "compile")]
            Subcommand::Compile(c) => c.execute(),

            #[cfg(feature = "explore")]
            Subcommand::Explore(c) => c.execute(),

            #[cfg(feature = "serve")]
            Subcommand::Serve(c) => c.execute(),

            #[cfg(feature = "cranelift")]
            Subcommand::Settings(c) => c.execute(),

            #[cfg(feature = "wast")]
            Subcommand::Wast(c) => c.execute(),

            #[cfg(feature = "completion")]
            Subcommand::Completion(c) => c.execute(),
        }
    }
}

/// Generate shell completion scripts for this CLI.
///
/// Shells have different paths for their completion scripts. Please refer to
/// their documentation. For example, to generate completions for the fish
/// shell, run the following command below:
///
///     wasmtime completion fish > ~/.config/fish/completions/wasmtime.fish
///
/// For a shell like zsh you can add this to your .zshrc or startup scripts:
///
///     eval "$(wasmtime completion zsh)"
#[derive(Parser)]
#[cfg(feature = "completion")]
pub struct CompletionCommand {
    /// The shell to generate completions for.
    shell: clap_complete::Shell,
}

#[cfg(feature = "completion")]
impl CompletionCommand {
    pub fn execute(&self) -> Result<()> {
        use clap::CommandFactory;

        let mut cmd = Wasmtime::command();
        let cli_name = cmd.get_name().to_owned();

        clap_complete::generate(self.shell, &mut cmd, cli_name, &mut std::io::stdout());
        Ok(())
    }
}

fn main() -> Result<()> {
    return Wasmtime::parse().execute();
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Wasmtime::command().debug_assert()
}
