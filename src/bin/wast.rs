//! The `wast` command line tool.
//!
//! Runs WebAssembly test script files.
//! See `wast --help` for usage.

use anyhow::Result;
use structopt::StructOpt;
use wasmtime_cli::commands::WastCommand;

fn main() -> Result<()> {
    WastCommand::from_args().execute()
}
