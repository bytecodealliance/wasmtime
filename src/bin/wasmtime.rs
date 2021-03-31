//! The `wasmtime` command line tool.
//!
//! Primarily used to run WebAssembly modules.
//! See `wasmtime --help` for usage.

use anyhow::Result;
use structopt::{clap::AppSettings, clap::ErrorKind, StructOpt};
use wasmtime_cli::commands::{
    CompileCommand, ConfigCommand, RunCommand, WasmToObjCommand, WastCommand,
};

/// Wasmtime WebAssembly Runtime
#[derive(StructOpt)]
#[structopt(
    name = "wasmtime",
    version = env!("CARGO_PKG_VERSION"),
    global_settings = &[
        AppSettings::VersionlessSubcommands,
        AppSettings::ColoredHelp
    ],
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
enum WasmtimeApp {
    // !!! IMPORTANT: if subcommands are added or removed, update `parse_module` in `src/commands/run.rs`. !!!
    /// Controls Wasmtime configuration settings
    Config(ConfigCommand),
    /// Compiles a WebAssembly module.
    Compile(CompileCommand),
    /// Runs a WebAssembly module
    Run(RunCommand),
    /// Translates a WebAssembly module to native object file
    #[structopt(name = "wasm2obj")]
    WasmToObj(WasmToObjCommand),
    /// Runs a WebAssembly test script file
    Wast(WastCommand),
}

impl WasmtimeApp {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        match self {
            Self::Config(c) => c.execute(),
            Self::Compile(c) => c.execute(),
            Self::Run(c) => c.execute(),
            Self::WasmToObj(c) => c.execute(),
            Self::Wast(c) => c.execute(),
        }
    }
}

fn main() -> Result<()> {
    WasmtimeApp::from_iter_safe(std::env::args())
        .unwrap_or_else(|e| match e.kind {
            ErrorKind::HelpDisplayed
            | ErrorKind::VersionDisplayed
            | ErrorKind::MissingArgumentOrSubcommand => e.exit(),
            _ => WasmtimeApp::Run(
                RunCommand::from_iter_safe(std::env::args()).unwrap_or_else(|_| e.exit()),
            ),
        })
        .execute()
}
