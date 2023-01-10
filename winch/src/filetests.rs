use std::process::Command;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Options {
    #[clap(last = true, value_parser)]
    cargo_test_args: Vec<String>,
}

// DOIT: document the fact that this is a wrapper around cargo test for the filetests crate
pub fn run(opts: &Options) -> Result<()> {
    Command::new("cargo")
        .arg("test")
        .arg("--package")
        .arg("winch-filetests")
        .arg("--")
        .args(&opts.cargo_test_args)
        .spawn()?
        .wait()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("Failed to run cargo test: {}", e))
}
