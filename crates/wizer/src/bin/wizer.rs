use anyhow::Context;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use structopt::StructOpt;
use wizer::Wizer;

#[derive(StructOpt)]
pub struct Options {
    /// The input Wasm module's file path.
    ///
    /// If not specified, then `stdin` is used.
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    /// The file path to write the output Wasm module to.
    ///
    /// If not specified, then `stdout` is used.
    #[structopt(short = "o", parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(flatten)]
    wizer: Wizer,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let options = Options::from_args();

    let stdin = io::stdin();
    let mut input: Box<dyn BufRead> = if let Some(input) = options.input.as_ref() {
        Box::new(io::BufReader::new(
            fs::File::open(input).context("failed to open input file")?,
        ))
    } else {
        Box::new(stdin.lock())
    };

    let mut output: Box<dyn Write> = if let Some(output) = options.output.as_ref() {
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

    let output_wasm = options.wizer.run(&input_wasm)?;

    output
        .write_all(&output_wasm)
        .context("failed to write to output")?;

    Ok(())
}
