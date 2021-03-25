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
        clippy::map_unwrap_or,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

pub mod commands;
mod obj;

use anyhow::{bail, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use target_lexicon::Triple;
use wasmtime::{Config, ProfilingStrategy, Strategy};

pub use obj::compile_to_obj;

fn pick_compilation_strategy(cranelift: bool, lightbeam: bool) -> Result<Strategy> {
    Ok(match (lightbeam, cranelift) {
        (true, false) => Strategy::Lightbeam,
        (false, true) => Strategy::Cranelift,
        (false, false) => Strategy::Auto,
        (true, true) => bail!("Can't enable --cranelift and --lightbeam at the same time"),
    })
}

fn pick_profiling_strategy(jitdump: bool, vtune: bool) -> Result<ProfilingStrategy> {
    Ok(match (jitdump, vtune) {
        (true, false) => ProfilingStrategy::JitDump,
        (false, true) => ProfilingStrategy::VTune,
        (true, true) => {
            println!("Can't enable --jitdump and --vtune at the same time. Profiling not enabled.");
            ProfilingStrategy::None
        }
        _ => ProfilingStrategy::None,
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

    /// Disable logging.
    #[structopt(long, conflicts_with = "log_to_files")]
    disable_logging: bool,

    /// Log to per-thread log files instead of stderr.
    #[structopt(long)]
    log_to_files: bool,

    /// Generate debug information
    #[structopt(short = "g")]
    debug_info: bool,

    /// Disable cache system
    #[structopt(long)]
    disable_cache: bool,

    /// Enable support for proposed SIMD instructions
    #[structopt(long)]
    enable_simd: bool,

    /// Disable support for reference types
    #[structopt(long)]
    disable_reference_types: bool,

    /// Disable support for multi-value functions
    #[structopt(long)]
    disable_multi_value: bool,

    /// Enable support for Wasm threads
    #[structopt(long)]
    enable_threads: bool,

    /// Disable support for bulk memory instructions
    #[structopt(long)]
    disable_bulk_memory: bool,

    /// Enable support for the multi-memory proposal
    #[structopt(long)]
    enable_multi_memory: bool,

    /// Enable support for the module-linking proposal
    #[structopt(long)]
    enable_module_linking: bool,

    /// Enable all experimental Wasm features
    #[structopt(long)]
    enable_all: bool,

    /// Use Lightbeam for all compilation
    #[structopt(long, conflicts_with = "cranelift")]
    lightbeam: bool,

    /// Generate jitdump file (supported on --features=profiling build)
    #[structopt(long, conflicts_with = "vtune")]
    jitdump: bool,

    /// Generate vtune (supported on --features=vtune build)
    #[structopt(long, conflicts_with = "jitdump")]
    vtune: bool,

    /// Run optimization passes on translated functions, on by default
    #[structopt(short = "O", long)]
    optimize: bool,

    /// Optimization level for generated functions: 0 (none), 1, 2 (most), or s
    /// (size); defaults to "most"
    #[structopt(
        long,
        value_name = "LEVEL",
        parse(try_from_str = parse_opt_level),
    )]
    opt_level: Option<wasmtime::OptLevel>,

    /// Cranelift common flags to set.
    #[structopt(long = "cranelift-flag", value_name = "NAME=VALUE", parse(try_from_str = parse_cranelift_flag))]
    cranelift_flags: Vec<CraneliftFlag>,

    /// The Cranelift ISA preset to use.
    #[structopt(long, value_name = "PRESET")]
    cranelift_preset: Option<String>,

    /// Maximum size in bytes of wasm memory before it becomes dynamically
    /// relocatable instead of up-front-reserved.
    #[structopt(long, value_name = "MAXIMUM")]
    static_memory_maximum_size: Option<u64>,

    /// Byte size of the guard region after static memories are allocated.
    #[structopt(long, value_name = "SIZE")]
    static_memory_guard_size: Option<u64>,

    /// Byte size of the guard region after dynamic memories are allocated.
    #[structopt(long, value_name = "SIZE")]
    dynamic_memory_guard_size: Option<u64>,

    /// Enable Cranelift's internal debug verifier (expensive)
    #[structopt(long)]
    enable_cranelift_debug_verifier: bool,

    /// Enable Cranelift's internal NaN canonicalization
    #[structopt(long)]
    enable_cranelift_nan_canonicalization: bool,
}

impl CommonOptions {
    fn config(&self, target: Option<&str>) -> Result<Config> {
        let mut config = if let Some(target) = target {
            Config::for_target(target)?
        } else {
            Config::new()
        };

        config
            .cranelift_debug_verifier(self.enable_cranelift_debug_verifier)
            .debug_info(self.debug_info)
            .wasm_simd(self.enable_simd || self.enable_all)
            .wasm_bulk_memory(!self.disable_bulk_memory || self.enable_all)
            .wasm_reference_types(
                (!self.disable_reference_types || cfg!(target_arch = "x86_64")) || self.enable_all,
            )
            .wasm_multi_value(!self.disable_multi_value || self.enable_all)
            .wasm_threads(self.enable_threads || self.enable_all)
            .wasm_multi_memory(self.enable_multi_memory || self.enable_all)
            .wasm_module_linking(self.enable_module_linking || self.enable_all)
            .cranelift_opt_level(self.opt_level())
            .strategy(pick_compilation_strategy(self.cranelift, self.lightbeam)?)?
            .profiler(pick_profiling_strategy(self.jitdump, self.vtune)?)?
            .cranelift_nan_canonicalization(self.enable_cranelift_nan_canonicalization);

        if let Some(preset) = &self.cranelift_preset {
            unsafe {
                config.cranelift_flag_enable(preset)?;
            }
        }

        for CraneliftFlag { name, value } in &self.cranelift_flags {
            unsafe {
                config.cranelift_other_flag(name, value)?;
            }
        }

        if !self.disable_cache {
            match &self.config {
                Some(path) => {
                    config.cache_config_load(path)?;
                }
                None => {
                    config.cache_config_load_default()?;
                }
            }
        }
        if let Some(max) = self.static_memory_maximum_size {
            config.static_memory_maximum_size(max);
        }
        if let Some(size) = self.static_memory_guard_size {
            config.static_memory_guard_size(size);
        }
        if let Some(size) = self.dynamic_memory_guard_size {
            config.dynamic_memory_guard_size(size);
        }
        Ok(config)
    }

    fn opt_level(&self) -> wasmtime::OptLevel {
        match (self.optimize, self.opt_level.clone()) {
            (true, _) => wasmtime::OptLevel::Speed,
            (false, other) => other.unwrap_or(wasmtime::OptLevel::Speed),
        }
    }
}

fn parse_opt_level(opt_level: &str) -> Result<wasmtime::OptLevel> {
    match opt_level {
        "s" => Ok(wasmtime::OptLevel::SpeedAndSize),
        "0" => Ok(wasmtime::OptLevel::None),
        "1" => Ok(wasmtime::OptLevel::Speed),
        "2" => Ok(wasmtime::OptLevel::Speed),
        other => bail!(
            "unknown optimization level `{}`, only 0,1,2,s accepted",
            other
        ),
    }
}

struct CraneliftFlag {
    name: String,
    value: String,
}

fn parse_cranelift_flag(name_and_value: &str) -> Result<CraneliftFlag> {
    let mut split = name_and_value.splitn(2, '=');
    let name = if let Some(name) = split.next() {
        name.to_string()
    } else {
        bail!("missing name in cranelift flag");
    };
    let value = if let Some(value) = split.next() {
        value.to_string()
    } else {
        bail!("missing value in cranelift flag");
    };
    Ok(CraneliftFlag { name, value })
}

fn parse_target(s: &str) -> Result<Triple> {
    use std::str::FromStr;

    Triple::from_str(&s).map_err(|e| anyhow::anyhow!(e))
}
