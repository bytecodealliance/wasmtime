use crate::commands::run::{CliInstance, RunCommand};
use crate::common::{RunCommon, RunTarget};
use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use wasmtime::Module;
use wasmtime_wizer::Wizer;

#[derive(clap::Parser)]
#[expect(missing_docs, reason = "inheriting wizer's docs")]
pub struct WizerCommand {
    #[command(flatten)]
    run: RunCommon,

    #[command(flatten)]
    wizer: Wizer,

    /// The input Wasm module's file path.
    input: PathBuf,

    /// The file path to write the output Wasm module to.
    ///
    /// If not specified, then `stdout` is used.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
}

impl WizerCommand {
    /// Runs the command.
    pub fn execute(mut self) -> Result<()> {
        self.run.common.init_logging()?;

        // By default use deterministic relaxed simd operations to guarantee
        // that if relaxed simd operations are used in a module that they always
        // produce the same result.
        if self.run.common.wasm.relaxed_simd_deterministic.is_none() {
            self.run.common.wasm.relaxed_simd_deterministic = Some(true);
        }

        // Read the input wasm, possibly from stdin.
        let mut wasm = Vec::new();
        if self.input.to_str() == Some("-") {
            io::stdin()
                .read_to_end(&mut wasm)
                .context("failed to read input Wasm module from stdin")?;
        } else {
            wasm = fs::read(&self.input).context("failed to read input Wasm module")?;
        }

        #[cfg(feature = "wat")]
        let wasm = wat::parse_bytes(&wasm)?;

        // Instrument the input wasm with wizer.
        let (cx, instrumented_wasm) = self.wizer.instrument(&wasm)?;

        // Execute a rough equivalent of
        // `wasmtime run --invoke <..> <instrumented-wasm>`
        let mut run = RunCommand {
            run: self.run,
            argv0: None,
            invoke: Some(self.wizer.get_init_func().to_string()),
            module_and_args: vec![self.input.clone().into()],
            preloads: Vec::new(), // TODO
        };
        let engine = run.new_engine()?;
        let main = RunTarget::Core(Module::new(&engine, &instrumented_wasm)?);
        let (mut store, mut linker) = run.new_store_and_linker(&engine, &main)?;
        #[allow(
            irrefutable_let_patterns,
            reason = "infallible when components are disabled"
        )]
        let CliInstance::Core(instance) =
            run.instantiate_and_run(&engine, &mut linker, &main, &mut store)?
        else {
            unreachable!()
        };

        // Use our state to capture a snapshot with Wizer and then serialize
        // that.
        let final_wasm = self.wizer.snapshot(cx, &mut store, &instance)?;

        match &self.output {
            Some(file) => fs::write(file, &final_wasm).context("failed to write output file")?,
            None => std::io::stdout()
                .write_all(&final_wasm)
                .context("failed to write output to stdout")?,
        }
        Ok(())
    }
}
