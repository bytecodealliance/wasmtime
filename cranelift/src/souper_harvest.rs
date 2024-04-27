use crate::utils::{iterate_files, read_to_string};
use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::Function;
use cranelift_codegen::Context;
use cranelift_reader::parse_sets_and_triple;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

/// Harvest candidates for superoptimization from a Wasm or Clif file.
///
/// Candidates are emitted in Souper's text format:
/// <https://github.com/google/souper>
#[derive(Parser)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    input: Vec<PathBuf>,

    /// Specify the directory where harvested left-hand side files should be
    /// written to.
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Configure Cranelift settings
    #[arg(long = "set")]
    settings: Vec<String>,

    /// Specify the Cranelift target
    #[arg(long = "target")]
    target: String,

    /// Add a comment from which CLIF variable and function each left-hand side
    /// was harvested from. This prevents deduplicating harvested left-hand
    /// sides.
    #[arg(long)]
    add_harvest_source: bool,
}

pub fn run(options: &Options) -> Result<()> {
    let parsed = parse_sets_and_triple(&options.settings, &options.target)?;
    let fisa = parsed.as_fisa();
    if fisa.isa.is_none() {
        anyhow::bail!("`souper-harvest` requires a target isa");
    }

    match fs::create_dir_all(&options.output_dir) {
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
                let hash = hash(lhs.as_bytes());
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

    iterate_files(&options.input)
        .par_bridge()
        .flat_map(|path| {
            parse_input(path)
                .unwrap_or_else(|e| {
                    println!("{e:?}");
                    Vec::new()
                })
                .into_par_iter()
        })
        .map_init(
            move || (send.clone(), Context::new()),
            move |(send, ctx), func| {
                ctx.clear();
                ctx.func = func;

                ctx.optimize(fisa.isa.unwrap(), &mut ControlPlane::default())
                    .context("failed to run optimizations")?;

                ctx.souper_harvest(send)
                    .context("failed to run souper harvester")?;

                Ok(())
            },
        )
        .collect::<Result<()>>()?;

    match writing_thread.join() {
        Ok(result) => result?,
        Err(e) => std::panic::resume_unwind(e),
    }

    Ok(())
}

fn parse_input(path: PathBuf) -> Result<Vec<Function>> {
    let contents = read_to_string(&path)?;
    let funcs = cranelift_reader::parse_functions(&contents)
        .with_context(|| format!("parse error in {}", path.display()))?;
    Ok(funcs)
}

/// A convenience functon for a quick usize hash
#[inline]
pub fn hash<T: std::hash::Hash + ?Sized>(v: &T) -> usize {
    let mut state = rustc_hash::FxHasher::default();
    v.hash(&mut state);
    std::hash::Hasher::finish(&state) as usize
}
