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

const SUPPORTED_WASM_FEATURES: &[(&str, &str)] = &[
    ("all", "enables all supported WebAssembly features"),
    (
        "bulk-memory",
        "enables support for bulk memory instructions",
    ),
    (
        "module-linking",
        "enables support for the module-linking proposal",
    ),
    (
        "multi-memory",
        "enables support for the multi-memory proposal",
    ),
    ("multi-value", "enables support for multi-value functions"),
    ("reference-types", "enables support for reference types"),
    ("simd", "enables support for proposed SIMD instructions"),
    ("threads", "enables support for WebAssembly threads"),
    ("memory64", "enables support for 64-bit memories"),
];

const SUPPORTED_WASI_MODULES: &[(&str, &str)] = &[
    (
        "default",
        "enables all stable WASI modules (no experimental modules)",
    ),
    (
        "wasi-common",
        "enables support for the WASI common APIs, see https://github.com/WebAssembly/WASI",
    ),
    (
        "experimental-wasi-nn",
        "enables support for the WASI neural network API (experimental), see https://github.com/WebAssembly/wasi-nn",
    ),
    (
        "experimental-wasi-crypto",
        "enables support for the WASI cryptography APIs (experimental), see https://github.com/WebAssembly/wasi-crypto",
    ),
];

lazy_static::lazy_static! {
    static ref FLAG_EXPLANATIONS: String = {
        use std::fmt::Write;

        let mut s = String::new();

        // Explain --wasm-features.
        writeln!(&mut s, "Supported values for `--wasm-features`:").unwrap();
        writeln!(&mut s).unwrap();
        let max = SUPPORTED_WASM_FEATURES.iter().max_by_key(|(name, _)| name.len()).unwrap();
        for (name, desc) in SUPPORTED_WASM_FEATURES.iter() {
            writeln!(&mut s, "{:width$} {}", name, desc, width = max.0.len() + 2).unwrap();
        }
        writeln!(&mut s).unwrap();

        // Explain --wasi-modules.
        writeln!(&mut s, "Supported values for `--wasi-modules`:").unwrap();
        writeln!(&mut s).unwrap();
        let max = SUPPORTED_WASI_MODULES.iter().max_by_key(|(name, _)| name.len()).unwrap();
        for (name, desc) in SUPPORTED_WASI_MODULES.iter() {
            writeln!(&mut s, "{:width$} {}", name, desc, width = max.0.len() + 2).unwrap();
        }

        writeln!(&mut s).unwrap();
        writeln!(&mut s, "Features prefixed with '-' will be disabled.").unwrap();

        s
    };
}

pub mod commands;
mod obj;

use anyhow::{bail, Result};
use std::collections::HashMap;
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

    /// Enable support for proposed SIMD instructions (deprecated; use `--wasm-features=simd`)
    #[structopt(long, hidden = true)]
    enable_simd: bool,

    /// Enable support for reference types (deprecated; use `--wasm-features=reference-types`)
    #[structopt(long, hidden = true)]
    enable_reference_types: bool,

    /// Enable support for multi-value functions (deprecated; use `--wasm-features=multi-value`)
    #[structopt(long, hidden = true)]
    enable_multi_value: bool,

    /// Enable support for Wasm threads (deprecated; use `--wasm-features=threads`)
    #[structopt(long, hidden = true)]
    enable_threads: bool,

    /// Enable support for bulk memory instructions (deprecated; use `--wasm-features=bulk-memory`)
    #[structopt(long, hidden = true)]
    enable_bulk_memory: bool,

    /// Enable support for the multi-memory proposal (deprecated; use `--wasm-features=multi-memory`)
    #[structopt(long, hidden = true)]
    enable_multi_memory: bool,

    /// Enable support for the module-linking proposal (deprecated; use `--wasm-features=module-linking`)
    #[structopt(long, hidden = true)]
    enable_module_linking: bool,

    /// Enable all experimental Wasm features (deprecated; use `--wasm-features=all`)
    #[structopt(long, hidden = true)]
    enable_all: bool,

    /// Enables or disables WebAssembly features
    #[structopt(long, value_name = "FEATURE,FEATURE,...", parse(try_from_str = parse_wasm_features))]
    wasm_features: Option<wasmparser::WasmFeatures>,

    /// Enables or disables WASI modules
    #[structopt(long, value_name = "MODULE,MODULE,...", parse(try_from_str = parse_wasi_modules))]
    wasi_modules: Option<WasiModules>,

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

    /// Optimization level for generated functions
    /// Supported levels: 0 (none), 1, 2 (most), or s (size); default is "most"
    #[structopt(
        long,
        value_name = "LEVEL",
        parse(try_from_str = parse_opt_level),
        verbatim_doc_comment,
    )]
    opt_level: Option<wasmtime::OptLevel>,

    /// Set a Cranelift setting to a given value.
    /// Use `wasmtime settings` to list Cranelift settings for a target.
    #[structopt(long = "cranelift-set", value_name = "NAME=VALUE", number_of_values = 1, verbatim_doc_comment, parse(try_from_str = parse_cranelift_flag))]
    cranelift_set: Vec<(String, String)>,

    /// Enable a Cranelift boolean setting or preset.
    /// Use `wasmtime settings` to list Cranelift settings for a target.
    #[structopt(
        long,
        value_name = "SETTING",
        number_of_values = 1,
        verbatim_doc_comment
    )]
    cranelift_enable: Vec<String>,

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
    fn init_logging(&self) {
        if self.disable_logging {
            return;
        }
        if self.log_to_files {
            let prefix = "wasmtime.dbg.";
            init_file_per_thread_logger(prefix);
        } else {
            pretty_env_logger::init();
        }
    }

    fn config(&self, target: Option<&str>) -> Result<Config> {
        let mut config = Config::new();

        // Set the target before setting any cranelift options
        if let Some(target) = target {
            config.target(target)?;
        }

        config
            .strategy(pick_compilation_strategy(self.cranelift, self.lightbeam)?)?
            .cranelift_debug_verifier(self.enable_cranelift_debug_verifier)
            .debug_info(self.debug_info)
            .cranelift_opt_level(self.opt_level())
            .profiler(pick_profiling_strategy(self.jitdump, self.vtune)?)?
            .cranelift_nan_canonicalization(self.enable_cranelift_nan_canonicalization);

        self.enable_wasm_features(&mut config);

        for name in &self.cranelift_enable {
            unsafe {
                config.cranelift_flag_enable(name)?;
            }
        }

        for (name, value) in &self.cranelift_set {
            unsafe {
                config.cranelift_flag_set(name, value)?;
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

    fn enable_wasm_features(&self, config: &mut Config) {
        let features = self.wasm_features.unwrap_or_default();

        config
            .wasm_simd(features.simd || self.enable_simd || self.enable_all)
            .wasm_bulk_memory(features.bulk_memory || self.enable_bulk_memory || self.enable_all)
            .wasm_reference_types(
                features.reference_types || self.enable_reference_types || self.enable_all,
            )
            .wasm_multi_value(features.multi_value || self.enable_multi_value || self.enable_all)
            .wasm_threads(features.threads || self.enable_threads || self.enable_all)
            .wasm_multi_memory(features.multi_memory || self.enable_multi_memory || self.enable_all)
            .wasm_memory64(features.memory64 || self.enable_all)
            .wasm_module_linking(
                features.module_linking || self.enable_module_linking || self.enable_all,
            );
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

fn parse_wasm_features(features: &str) -> Result<wasmparser::WasmFeatures> {
    let features = features.trim();

    let mut all = None;
    let mut values: HashMap<_, _> = SUPPORTED_WASM_FEATURES
        .iter()
        .map(|(name, _)| (name.to_string(), None))
        .collect();

    if features == "all" {
        all = Some(true);
    } else if features == "-all" {
        all = Some(false);
    } else {
        for feature in features.split(',') {
            let feature = feature.trim();

            if feature.is_empty() {
                continue;
            }

            let (feature, value) = if feature.starts_with('-') {
                (&feature[1..], false)
            } else {
                (feature, true)
            };

            if feature == "all" {
                bail!("'all' cannot be specified with other WebAssembly features");
            }

            match values.get_mut(feature) {
                Some(v) => *v = Some(value),
                None => bail!("unsupported WebAssembly feature '{}'", feature),
            }
        }
    }

    Ok(wasmparser::WasmFeatures {
        reference_types: all.unwrap_or(values["reference-types"].unwrap_or(true)),
        multi_value: all.unwrap_or(values["multi-value"].unwrap_or(true)),
        bulk_memory: all.unwrap_or(values["bulk-memory"].unwrap_or(true)),
        module_linking: all.unwrap_or(values["module-linking"].unwrap_or(false)),
        simd: all.unwrap_or(values["simd"].unwrap_or(false)),
        threads: all.unwrap_or(values["threads"].unwrap_or(false)),
        tail_call: false,
        deterministic_only: false,
        multi_memory: all.unwrap_or(values["multi-memory"].unwrap_or(false)),
        exceptions: false,
        memory64: all.unwrap_or(values["memory64"].unwrap_or(false)),
    })
}

fn parse_wasi_modules(modules: &str) -> Result<WasiModules> {
    let modules = modules.trim();
    match modules {
        "default" => Ok(WasiModules::default()),
        "-default" => Ok(WasiModules::none()),
        _ => {
            // Starting from the default set of WASI modules, enable or disable a list of
            // comma-separated modules.
            let mut wasi_modules = WasiModules::default();
            let mut set = |module: &str, enable: bool| match module {
                "" => Ok(()),
                "wasi-common" => Ok(wasi_modules.wasi_common = enable),
                "experimental-wasi-nn" => Ok(wasi_modules.wasi_nn = enable),
                "experimental-wasi-crypto" => Ok(wasi_modules.wasi_crypto = enable),
                "default" => bail!("'default' cannot be specified with other WASI modules"),
                _ => bail!("unsupported WASI module '{}'", module),
            };

            for module in modules.split(',') {
                let module = module.trim();
                let (module, value) = if module.starts_with('-') {
                    (&module[1..], false)
                } else {
                    (module, true)
                };
                set(module, value)?;
            }

            Ok(wasi_modules)
        }
    }
}

/// Select which WASI modules are available at runtime for use by Wasm programs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WasiModules {
    /// Enable the wasi-common implementation; eventually this should be split into its separate
    /// parts once the implementation allows for it (e.g. wasi-fs, wasi-clocks, etc.).
    pub wasi_common: bool,

    /// Enable the experimental wasi-nn implementation.
    pub wasi_nn: bool,

    /// Enable the experimental wasi-crypto implementation.
    pub wasi_crypto: bool,
}

impl Default for WasiModules {
    fn default() -> Self {
        Self {
            wasi_common: true,
            wasi_nn: false,
            wasi_crypto: false,
        }
    }
}

impl WasiModules {
    /// Enable no modules.
    pub fn none() -> Self {
        Self {
            wasi_common: false,
            wasi_nn: false,
            wasi_crypto: false,
        }
    }
}

fn parse_cranelift_flag(name_and_value: &str) -> Result<(String, String)> {
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
    Ok((name, value))
}

fn parse_target(s: &str) -> Result<Triple> {
    use std::str::FromStr;
    Triple::from_str(&s).map_err(|e| anyhow::anyhow!(e))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_all_features() -> Result<()> {
        let options = CommonOptions::from_iter_safe(vec!["foo", "--wasm-features=all"])?;

        let wasmparser::WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            module_linking,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
        } = options.wasm_features.unwrap();

        assert!(reference_types);
        assert!(multi_value);
        assert!(bulk_memory);
        assert!(module_linking);
        assert!(simd);
        assert!(threads);
        assert!(!tail_call); // Not supported
        assert!(!deterministic_only); // Not supported
        assert!(multi_memory);
        assert!(!exceptions); // Not supported
        assert!(memory64);

        Ok(())
    }

    #[test]
    fn test_no_features() -> Result<()> {
        let options = CommonOptions::from_iter_safe(vec!["foo", "--wasm-features=-all"])?;

        let wasmparser::WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            module_linking,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
        } = options.wasm_features.unwrap();

        assert!(!reference_types);
        assert!(!multi_value);
        assert!(!bulk_memory);
        assert!(!module_linking);
        assert!(!simd);
        assert!(!threads);
        assert!(!tail_call);
        assert!(!deterministic_only);
        assert!(!multi_memory);
        assert!(!exceptions);
        assert!(!memory64);

        Ok(())
    }

    #[test]
    fn test_multiple_features() -> Result<()> {
        let options = CommonOptions::from_iter_safe(vec![
            "foo",
            "--wasm-features=-reference-types,simd,multi-memory,memory64",
        ])?;

        let wasmparser::WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            module_linking,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
        } = options.wasm_features.unwrap();

        assert!(!reference_types);
        assert!(multi_value);
        assert!(bulk_memory);
        assert!(!module_linking);
        assert!(simd);
        assert!(!threads);
        assert!(!tail_call); // Not supported
        assert!(!deterministic_only); // Not supported
        assert!(multi_memory);
        assert!(!exceptions); // Not supported
        assert!(memory64);

        Ok(())
    }

    macro_rules! feature_test {
        ($test_name:ident, $name:ident, $flag:literal) => {
            #[test]
            fn $test_name() -> Result<()> {
                let options =
                    CommonOptions::from_iter_safe(vec!["foo", concat!("--wasm-features=", $flag)])?;

                let wasmparser::WasmFeatures { $name, .. } = options.wasm_features.unwrap();

                assert!($name);

                let options = CommonOptions::from_iter_safe(vec![
                    "foo",
                    concat!("--wasm-features=-", $flag),
                ])?;

                let wasmparser::WasmFeatures { $name, .. } = options.wasm_features.unwrap();

                assert!(!$name);

                Ok(())
            }
        };
    }

    feature_test!(
        test_reference_types_feature,
        reference_types,
        "reference-types"
    );
    feature_test!(test_multi_value_feature, multi_value, "multi-value");
    feature_test!(test_bulk_memory_feature, bulk_memory, "bulk-memory");
    feature_test!(
        test_module_linking_feature,
        module_linking,
        "module-linking"
    );
    feature_test!(test_simd_feature, simd, "simd");
    feature_test!(test_threads_feature, threads, "threads");
    feature_test!(test_multi_memory_feature, multi_memory, "multi-memory");
    feature_test!(test_memory64_feature, memory64, "memory64");

    #[test]
    fn test_default_modules() {
        let options = CommonOptions::from_iter_safe(vec!["foo", "--wasi-modules=default"]).unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: true,
                wasi_nn: false,
                wasi_crypto: false
            }
        );
    }

    #[test]
    fn test_empty_modules() {
        let options = CommonOptions::from_iter_safe(vec!["foo", "--wasi-modules="]).unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: true,
                wasi_nn: false,
                wasi_crypto: false
            }
        );
    }

    #[test]
    fn test_some_modules() {
        let options = CommonOptions::from_iter_safe(vec![
            "foo",
            "--wasi-modules=experimental-wasi-nn,-wasi-common",
        ])
        .unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: false,
                wasi_nn: true,
                wasi_crypto: false
            }
        );
    }

    #[test]
    fn test_no_modules() {
        let options =
            CommonOptions::from_iter_safe(vec!["foo", "--wasi-modules=-default"]).unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: false,
                wasi_nn: false,
                wasi_crypto: false
            }
        );
    }
}
