use crate::utils::parse_sets_and_triple;
use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift_codegen::Context;
use cranelift_wasm::{DummyEnvironment, ReturnMode};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use std::{fs, io};

static WASM_MAGIC: &[u8] = &[0x00, 0x61, 0x73, 0x6D];

/// Harvest candidates for superoptimization from a Wasm or Clif file.
///
/// Candidates are emitted in Souper's text format:
/// <https://github.com/google/souper>
#[derive(Parser)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    input: PathBuf,

    /// Specify the output file to be used. Use '-' for stdout.
    #[clap(short, long, default_value("-"))]
    output: PathBuf,

    /// Configure Cranelift settings
    #[clap(long = "set")]
    settings: Vec<String>,

    /// Specify the Cranelift target
    #[clap(long = "target")]
    target: String,
}

pub fn run(options: &Options) -> Result<()> {
    let parsed = parse_sets_and_triple(&options.settings, &options.target)?;
    let fisa = parsed.as_fisa();
    if fisa.isa.is_none() {
        anyhow::bail!("`souper-harvest` requires a target isa");
    }

    let stdin = io::stdin();
    let mut input: Box<dyn io::BufRead> = if options.input == Path::new("-") {
        Box::new(stdin.lock())
    } else {
        Box::new(io::BufReader::new(
            fs::File::open(&options.input).context("failed to open input file")?,
        ))
    };

    let mut output: Box<dyn io::Write + Send> = if options.output == Path::new("-") {
        Box::new(io::stdout())
    } else {
        Box::new(io::BufWriter::new(
            fs::File::create(&options.output).context("failed to create output file")?,
        ))
    };

    let mut contents = vec![];
    input
        .read_to_end(&mut contents)
        .context("failed to read input file")?;

    let funcs = if &contents[..WASM_MAGIC.len()] == WASM_MAGIC {
        let mut dummy_environ = DummyEnvironment::new(
            fisa.isa.unwrap().frontend_config(),
            ReturnMode::NormalReturns,
            false,
        );
        cranelift_wasm::translate_module(&contents, &mut dummy_environ)
            .context("failed to translate Wasm module to clif")?;
        dummy_environ
            .info
            .function_bodies
            .iter()
            .map(|(_, f)| f.clone())
            .collect()
    } else {
        let contents = String::from_utf8(contents)?;
        cranelift_reader::parse_functions(&contents)?
    };

    let (send, recv) = std::sync::mpsc::channel::<String>();

    let writing_thread = std::thread::spawn(move || -> Result<()> {
        for lhs in recv {
            output
                .write_all(lhs.as_bytes())
                .context("failed to write to output file")?;
        }
        Ok(())
    });

    funcs
        .into_par_iter()
        .map_with(send, move |send, func| {
            let mut ctx = Context::new();
            ctx.func = func;

            ctx.compute_cfg();
            ctx.preopt(fisa.isa.unwrap())
                .context("failed to run preopt")?;

            ctx.souper_harvest(send)
                .context("failed to run souper harvester")?;

            Ok(())
        })
        .collect::<Result<()>>()?;

    match writing_thread.join() {
        Ok(result) => result?,
        Err(e) => std::panic::resume_unwind(e),
    }

    Ok(())
}
