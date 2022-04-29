//! The `wasmtime` command line tool.
//!
//! Primarily used to run WebAssembly modules.
//! See `wasmtime --help` for usage.

use anyhow::Result;
use clap::{ErrorKind, Parser};
use wasmtime_cli::commands::{
    CompileCommand, ConfigCommand, RunCommand, SettingsCommand, WastCommand,
};

/// Wasmtime WebAssembly Runtime
#[derive(Parser)]
#[clap(
    version,
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
                  wasmtime example.wasm --invoke add 1 2\n"
)]
enum Wasmtime {
    // !!! IMPORTANT: if subcommands are added or removed, update `parse_module` in `src/commands/run.rs`. !!!
    /// Controls Wasmtime configuration settings
    Config(ConfigCommand),
    /// Compiles a WebAssembly module.
    Compile(CompileCommand),
    /// Runs a WebAssembly module
    Run(RunCommand),
    /// Displays available Cranelift settings for a target.
    Settings(SettingsCommand),
    /// Runs a WebAssembly test script file
    Wast(WastCommand),
}

impl Wasmtime {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        match self {
            Self::Config(c) => c.execute(),
            Self::Compile(c) => c.execute(),
            Self::Run(c) => c.execute(),
            Self::Settings(c) => c.execute(),
            Self::Wast(c) => c.execute(),
        }
    }
}

fn main() -> Result<()> {
    Wasmtime::try_parse()
        .unwrap_or_else(|e| match e.kind() {
            ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::MissingSubcommand
            | ErrorKind::MissingRequiredArgument => e.exit(),
            _ => Wasmtime::Run(RunCommand::parse()),
        })
        .execute()
}
