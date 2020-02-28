//! The `wasm2obj` command line tool.
//!
//! Translates WebAssembly modules to object files.
//! See `wasm2obj --help` for usage.

use anyhow::Result;
use structopt::StructOpt;
use wasmtime_cli::commands::WasmToObjCommand;

fn main() -> Result<()> {
    WasmToObjCommand::from_args().execute()
}
