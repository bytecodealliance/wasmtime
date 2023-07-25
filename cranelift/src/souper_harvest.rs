use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift_codegen::Context;
use cranelift_reader::parse_sets_and_triple;
use cranelift_wasm::DummyEnvironment;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;
use std::io::Write;
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

    /// Specify the directory where harvested left-hand side files should be
    /// written to.
    #[clap(short, long)]
    output_dir: PathBuf,

    /// Configure Cranelift settings
    #[clap(long = "set")]
    settings: Vec<String>,

    /// Specify the Cranelift target
    #[clap(long = "target")]
    target: String,

    /// Add a comment from which CLIF variable and function each left-hand side
    /// was harvested from. This prevents deduplicating harvested left-hand
    /// sides.
    #[clap(long)]
    add_harvest_source: bool,
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

    match std::fs::create_dir_all(&options.output_dir) {
        Ok(_) => {}
        Err(e)
            if e.kind() == io::ErrorKind::AlreadyExists
                && fs::metadata(&options.output_dir)
                    .with_context(|| {
                        format!(
                            "failed to read file metadata: {}",
                            options.output_dir.display(),
                        )
                    })?
                    .is_dir() => {}
        Err(e) => {
            return Err(e).context(format!(
                "failed to create output directory: {}",
                options.output_dir.display()
            ))
        }
    }

    let mut contents = vec![];
    input
        .read_to_end(&mut contents)
        .context("failed to read input file")?;

    let funcs = if &contents[..WASM_MAGIC.len()] == WASM_MAGIC {
        let mut dummy_environ = DummyEnvironment::new(fisa.isa.unwrap().frontend_config());
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

    let writing_thread = std::thread::spawn({
        let output_dir = options.output_dir.clone();
        let keep_harvest_source = options.add_harvest_source;
        move || -> Result<()> {
            let mut already_harvested = HashSet::new();
            for lhs in recv {
                let lhs = if keep_harvest_source {
                    &lhs
                } else {
                    // Remove the first `;; Harvested from v12 in u:34` line.
                    let i = lhs.find('\n').unwrap();
                    &lhs[i + 1..]
                };
                let hash = fxhash::hash(lhs.as_bytes());
                if already_harvested.insert(hash) {
                    let output_path = output_dir.join(hash.to_string());
                    let mut output =
                        io::BufWriter::new(fs::File::create(&output_path).with_context(|| {
                            format!("failed to create file: {}", output_path.display())
                        })?);
                    output.write_all(lhs.as_bytes()).with_context(|| {
                        format!("failed to write to output file: {}", output_path.display())
                    })?;
                }
            }
            Ok(())
        }
    });

    funcs
        .into_par_iter()
        .map_with(send, move |send, func| {
            let mut ctx = Context::new();
            ctx.func = func;

            ctx.optimize(fisa.isa.unwrap())
                .context("failed to run optimizations")?;

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
