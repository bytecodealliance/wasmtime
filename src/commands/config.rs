//! The module that implements the `wasmtime config` command.

use anyhow::Result;
use clap::Parser;

const CONFIG_NEW_AFTER_HELP: &str =
    "If no file path is specified, the system configuration file path will be used.";

/// Controls Wasmtime configuration settings
#[derive(Parser)]
#[clap(name = "config")]
pub struct ConfigCommand {
    #[clap(subcommand)]
    subcommand: ConfigSubcommand,
}

#[derive(clap::Subcommand)]
enum ConfigSubcommand {
    /// Creates a new Wasmtime configuration file
    #[clap(after_help = CONFIG_NEW_AFTER_HELP)]
    New(ConfigNewCommand),
}

impl ConfigCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        match self.subcommand {
            ConfigSubcommand::New(c) => c.execute(),
        }
    }
}

/// Creates a new Wasmtime configuration file
#[derive(Parser)]
#[clap(name = "new", after_help = CONFIG_NEW_AFTER_HELP)]
pub struct ConfigNewCommand {
    /// The path of the new configuration file
    #[clap(index = 1, value_name = "FILE_PATH")]
    path: Option<String>,
}

impl ConfigNewCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        let path = wasmtime_cache::create_new_config(self.path.as_ref())?;

        println!(
            "Successfully created a new configuration file at '{}'.",
            path.display()
        );

        Ok(())
    }
}
