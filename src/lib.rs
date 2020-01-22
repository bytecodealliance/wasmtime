//! The Wasmtime command line interface (CLI) crate.
//!
//! This crate implements the Wasmtime command line tools.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

pub mod commands;

use anyhow::{bail, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use wasmtime::{Config, Strategy};

fn pick_compilation_strategy(cranelift: bool, lightbeam: bool) -> Result<Strategy> {
    Ok(match (lightbeam, cranelift) {
        (true, false) => Strategy::Lightbeam,
        (false, true) => Strategy::Cranelift,
        (false, false) => Strategy::Auto,
        (true, true) => bail!("Can't enable --cranelift and --lightbeam at the same time"),
    })
}

fn init_file_per_thread_logger(prefix: &'static str) {
    file_per_thread_logger::initialize(prefix);

    // Extending behavior of default spawner:
    // https://docs.rs/rayon/1.1.0/rayon/struct.ThreadPoolBuilder.html#method.spawn_handler
    // Source code says DefaultSpawner is implementation detail and
    // shouldn't be used directly.
    rayon::ThreadPoolBuilder::new()
        .spawn_handler(move |thread| {
            let mut b = std::thread::Builder::new();
            if let Some(name) = thread.name() {
                b = b.name(name.to_owned());
            }
            if let Some(stack_size) = thread.stack_size() {
                b = b.stack_size(stack_size);
            }
            b.spawn(move || {
                file_per_thread_logger::initialize(prefix);
                thread.run()
            })?;
            Ok(())
        })
        .build_global()
        .unwrap();
}

/// Common options for commands that translate WebAssembly modules
#[derive(StructOpt)]
struct CommonOptions {
    /// Use specified configuration file
    #[structopt(long, parse(from_os_str), value_name = "CONFIG_PATH")]
    config: Option<PathBuf>,

    /// Use Cranelift for all compilation
    #[structopt(long, conflicts_with = "lightbeam")]
    cranelift: bool,

    /// Enable debug output
    #[structopt(short, long)]
    debug: bool,

    /// Generate debug information
    #[structopt(short = "g")]
    debug_info: bool,

    /// Disable cache system
    #[structopt(long)]
    disable_cache: bool,

    /// Enable support for proposed SIMD instructions
    #[structopt(long)]
    enable_simd: bool,

    /// Use Lightbeam for all compilation
    #[structopt(long, conflicts_with = "cranelift")]
    lightbeam: bool,

    /// Run optimization passes on translated functions
    #[structopt(short = "O", long)]
    optimize: bool,
}

impl CommonOptions {
    fn configure_cache(&self, config: &mut Config) -> Result<()> {
        if self.disable_cache {
            return Ok(());
        }
        match &self.config {
            Some(path) => {
                config.cache_config_load(path)?;
            }
            None => {
                config.cache_config_load_default()?;
            }
        }
        Ok(())
    }
}
