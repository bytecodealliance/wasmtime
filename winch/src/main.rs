mod compile;
mod filetests;

use anyhow::Result;
use clap::Parser;

/// Winch compilation and testing tool.
#[derive(Parser)]
enum Commands {
    /// Compile a Wasm module to the specified target architecture.
    Compile(compile::Options),
    /// Run the filetests.
    Test(filetests::Options),
}

fn main() -> Result<()> {
    match Commands::parse() {
        Commands::Compile(c) => compile::run(&c),
        Commands::Test(t) => filetests::run(&t),
    }
}
