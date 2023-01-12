use std::process::Command;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Options {
    /// Passes extra arguments to `cargo test --package winch-filetests`. For example, to run a single
    /// test, use `-- --test-threads 1 --test single_test_name`.
    #[clap(last = true, value_parser)]
    cargo_test_args: Vec<String>,
}

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
