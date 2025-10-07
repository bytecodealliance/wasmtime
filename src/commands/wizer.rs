use anyhow::Context;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use wasmtime_wizer::Wizer;

#[allow(missing_docs)] // inherit the docs of the `Wizer` field
#[derive(clap::Parser)]
pub struct WizerCommand {
    /// The input Wasm module's file path.
    ///
    /// If not specified, then `stdin` is used.
    input: Option<PathBuf>,

    /// The file path to write the output Wasm module to.
    ///
    /// If not specified, then `stdout` is used.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    #[clap(flatten)]
    wizer: Wizer,
}

impl WizerCommand {
    /// Runs the command.
    pub fn execute(self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let mut input: Box<dyn BufRead> = if let Some(input) = self.input.as_ref() {
            Box::new(io::BufReader::new(
                fs::File::open(input).context("failed to open input file")?,
            ))
        } else {
            Box::new(stdin.lock())
        };

        let mut output: Box<dyn Write> = if let Some(output) = self.output.as_ref() {
            Box::new(io::BufWriter::new(
                fs::File::create(output).context("failed to create output file")?,
            ))
        } else {
            Box::new(io::stdout())
        };

        let mut input_wasm = vec![];
        input
            .read_to_end(&mut input_wasm)
            .context("failed to read input Wasm module")?;

        let output_wasm = self.wizer.run(&input_wasm)?;

        output
            .write_all(&output_wasm)
            .context("failed to write to output")?;

        Ok(())
    }
}
