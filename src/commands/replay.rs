//! Implementation of the `wasmtime replay` command

use crate::commands::run::RunCommand;
use anyhow::Result;
use clap::Parser;
use std::{fs, io::BufReader, path::PathBuf, sync::Arc};
use wasmtime::{ReplayConfig, ReplaySettings};

#[derive(Parser)]
/// Replay-specific options for CLI
pub struct ReplayOptions {
    /// The path of the recorded trace
    ///
    /// Execution traces can be obtained for most modes of Wasmtime execution with -R.
    /// See `wasmtime run -R help` for relevant information on recording execution
    ///
    /// Note: The module used for replay must exactly match that used during recording
    #[arg(short, long, required = true, value_name = "RECORDED TRACE")]
    trace: PathBuf,

    /// Dynamic checks of record signatures to validate replay consistency.
    ///
    /// Requires record traces to be generated with `validation_metadata` enabled.
    #[arg(short, long, default_value_t = false)]
    validate: bool,

    /// Size of static buffer needed to deserialized variable-length types like String. This is not
    /// not relevant for basic functional recording/replaying, but may be required to replay traces where
    /// `validation-metadata` was enabled for recording
    #[arg(short, long, default_value_t = 64)]
    deser_buffer_size: usize,
}

/// Execute a deterministic, embedding-agnostic replay of a Wasm modules given its associated recorded trace
#[derive(Parser)]
pub struct ReplayCommand {
    #[command(flatten)]
    replay_opts: ReplayOptions,

    #[command(flatten)]
    run_cmd: RunCommand,
}

impl ReplayCommand {
    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        #[cfg(not(feature = "rr-validate"))]
        if self.replay_opts.validate {
            anyhow::bail!("Cannot use `validate` when `rr-validate` feature is disabled");
        }
        let replay_cfg = ReplayConfig {
            reader_initializer: Arc::new(move || {
                Box::new(BufReader::new(
                    fs::File::open(&self.replay_opts.trace).unwrap(),
                ))
            }),
            settings: ReplaySettings {
                validate: self.replay_opts.validate,
                deser_buffer_size: self.replay_opts.deser_buffer_size,
                ..Default::default()
            },
        };
        // Replay uses the `run` command harness
        self.run_cmd.execute(Some(replay_cfg))
    }
}
