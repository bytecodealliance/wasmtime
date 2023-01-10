// DOIT: update this doc comment with the new functionality
//! Winch CLI tool, meant mostly for testing purposes.
//!
//! Reads Wasm in binary/text format and compiles them
//! to any of the supported architectures using Winch.

mod compile;
mod filetests;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
enum Commands {
    /// Compile a Wasm module to the specified target architecture.
    Compile(compile::Options),
    Test(filetests::Options),
}

fn main() -> Result<()> {
    match Commands::parse() {
        Commands::Compile(c) => compile::run(&c),
        Commands::Test(t) => filetests::run(&t),
    }
}
