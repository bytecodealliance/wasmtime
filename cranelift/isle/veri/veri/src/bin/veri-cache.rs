//! Maintenance operations on the ISLE verifier's S3-backed SMT query cache.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result, format_err};
use clap::{Parser, Subcommand};
use cranelift_isle_veri_caching::{Cache, CacheMode, Credentials, S3Store, write_seed_base};

#[derive(Parser)]
struct Opts {
    /// S3 bucket holding the cache.
    ///
    /// Requests are signed with credentials from `AWS_ACCESS_KEY_ID`,
    /// `AWS_SECRET_ACCESS_KEY` and (optionally) `AWS_SESSION_TOKEN`, which
    /// all subcommands require. The region is taken from `AWS_REGION` or
    /// `AWS_DEFAULT_REGION`.
    #[arg(long, env = "ISLE_VERI_CACHE_BUCKET")]
    bucket: String,

    /// Key prefix of the cache within the S3 bucket.
    #[arg(long, default_value = "v1/", env = "ISLE_VERI_CACHE_PREFIX")]
    prefix: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compact the cache if enough deltas have accumulated: write a new base
    /// holding only the entries listed in a used-keys file, and delete the
    /// base and deltas it supersedes.
    Compact {
        /// Used-keys file written (via `veri --cache-used-keys-file`) by the
        /// complete set of runs of one verification. Entries not listed are
        /// dropped.
        #[arg(long, value_name = "FILE")]
        used_keys_file: PathBuf,

        /// Compact only once there are at least this many deltas not covered
        /// by the current base.
        #[arg(
            long,
            default_value = "20",
            env = "ISLE_VERI_CACHE_COMPACTION_THRESHOLD"
        )]
        threshold: usize,
    },

    /// Upload a directory of old-format JSON cache entries (`*.json` files
    /// with `key_sha256` and `response` fields, as in the former
    /// `isle-veri-cache.tar.gz` release asset) as a new base.
    Import {
        /// Directory of JSON cache entries.
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let credentials = Credentials::from_env()
        .ok_or_else(|| format_err!("AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set"))?;
    let store = Arc::new(S3Store::new(&opts.bucket, &opts.prefix, Some(credentials)));

    match opts.command {
        Command::Compact {
            used_keys_file,
            threshold,
        } => {
            let cache = Cache::open(store, CacheMode::ReadWrite)
                .context("failed to load SMT query cache")?;
            let uncovered = cache.uncovered_deltas();
            match cache.compact_with_used_keys_file(&used_keys_file, threshold)? {
                None => println!(
                    "Cache: {uncovered} uncovered deltas (threshold {threshold}); not compacting"
                ),
                Some(summary) => println!(
                    "Cache: compacted {uncovered} deltas into new base {}: kept {} entries, \
                     dropped {} unused; {} superseded objects deleted",
                    summary.base, summary.kept, summary.dropped, summary.deleted
                ),
            }
        }

        Command::Import { dir } => {
            let mut entries = Vec::new();
            for dirent in std::fs::read_dir(&dir)? {
                let path = dirent?.path();
                if path.extension().is_none_or(|e| e != "json") {
                    continue;
                }
                let entry: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
                let key = entry["key_sha256"]
                    .as_str()
                    .ok_or_else(|| format_err!("{} has no key_sha256", path.display()))?
                    .to_string();
                entries.push((key, entry["response"].clone()));
            }
            let count = entries.len();
            let name = write_seed_base(&*store, entries)?;
            println!("Uploaded {count} entries as {name}");
        }
    }
    Ok(())
}
