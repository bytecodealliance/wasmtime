//! The module that implements the `wasmtime config` command.

use anyhow::Result;
use structopt::StructOpt;

const CONFIG_NEW_AFTER_HELP: &str =
    "If no file path is specified, the system configuration file path will be used.";

/// Controls Wasmtime configuration settings
#[derive(StructOpt)]
#[structopt(name = "run")]
pub enum ConfigCommand {
    /// Creates a new Wasmtime configuration file
    #[structopt(after_help = CONFIG_NEW_AFTER_HELP)]
    New(ConfigNewCommand),
}

impl ConfigCommand {
    /// Executes the command.
    pub fn execute(&self) -> Result<()> {
        match self {
            Self::New(c) => c.execute(),
        }
    }
}

/// Creates a new Wasmtime configuration file
#[derive(StructOpt)]
#[structopt(name = "new", after_help = CONFIG_NEW_AFTER_HELP)]
pub struct ConfigNewCommand {
    /// The path of the new configuration file
    #[structopt(index = 1, value_name = "FILE_PATH")]
    path: Option<String>,
}

impl ConfigNewCommand {
    /// Executes the command.
    pub fn execute(&self) -> Result<()> {
        let path = wasmtime_cache::create_new_config(self.path.as_ref())?;

        println!(
            "Successfully created a new configuation file at '{}'.",
            path.display()
        );

        Ok(())
    }
}
