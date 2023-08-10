//! Contains the common Wasmtime command line interface (CLI) flags.

#![deny(trivial_numeric_casts, unused_extern_crates, unstable_features)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
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

use anyhow::{bail, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use wasmtime::{Config, Strategy};

pub const SUPPORTED_WASM_FEATURES: &[(&str, &str)] = &[
    ("all", "enables all supported WebAssembly features"),
    (
        "bulk-memory",
        "enables support for bulk memory instructions",
    ),
    (
        "multi-memory",
        "enables support for the multi-memory proposal",
    ),
    ("multi-value", "enables support for multi-value functions"),
    ("reference-types", "enables support for reference types"),
    ("simd", "enables support for proposed SIMD instructions"),
    (
        "relaxed-simd",
        "enables support for the relaxed simd proposal",
    ),
    ("tail-call", "enables support for WebAssembly tail calls"),
    ("threads", "enables support for WebAssembly threads"),
    ("memory64", "enables support for 64-bit memories"),
    #[cfg(feature = "component-model")]
    ("component-model", "enables support for the component model"),
    (
        "function-references",
        "enables support for typed function references",
    ),
];

pub const SUPPORTED_WASI_MODULES: &[(&str, &str)] = &[
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
        "experimental-wasi-threads",
        "enables support for the WASI threading API (experimental), see https://github.com/WebAssembly/wasi-threads",
    ),
    (
        "experimental-wasi-http",
        "enables support for the WASI HTTP APIs (experimental), see https://github.com/WebAssembly/wasi-http",
    ),
];

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
#[derive(Parser)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CommonOptions {
    /// Use specified configuration file
    #[clap(long, value_name = "CONFIG_PATH")]
    pub config: Option<PathBuf>,

    /// Disable logging
    #[clap(long, conflicts_with = "log_to_files")]
    pub disable_logging: bool,

    /// Log to per-thread log files instead of stderr
    #[clap(long)]
    pub log_to_files: bool,

    /// Generate debug information
    #[clap(short = 'g')]
    pub debug_info: bool,

    /// Disable cache system
    #[clap(long)]
    pub disable_cache: bool,

    /// Disable parallel compilation
    #[clap(long)]
    pub disable_parallel_compilation: bool,

    /// Enable or disable WebAssembly features
    #[clap(long, value_name = "FEATURE,FEATURE,...", value_parser = parse_wasm_features)]
    pub wasm_features: Option<WasmFeatures>,

    /// Enable or disable WASI modules
    #[clap(long, value_name = "MODULE,MODULE,...", value_parser = parse_wasi_modules)]
    pub wasi_modules: Option<WasiModules>,

    /// Generate jitdump file (supported on --features=profiling build)
    /// Run optimization passes on translated functions, on by default
    #[clap(short = 'O', long)]
    pub optimize: bool,

    /// Optimization level for generated functions
    /// Supported levels: 0 (none), 1, 2 (most), or s (size); default is "most"
    #[clap(
        long,
        value_name = "LEVEL",
        value_parser = parse_opt_level,
        verbatim_doc_comment,
    )]
    pub opt_level: Option<wasmtime::OptLevel>,

    /// Set a Cranelift setting to a given value.
    /// Use `wasmtime settings` to list Cranelift settings for a target.
    #[clap(
        long = "cranelift-set",
        value_name = "NAME=VALUE",
        number_of_values = 1,
        verbatim_doc_comment,
        value_parser = parse_cranelift_flag,
    )]
    pub cranelift_set: Vec<(String, String)>,

    /// Enable a Cranelift boolean setting or preset.
    /// Use `wasmtime settings` to list Cranelift settings for a target.
    #[clap(
        long,
        value_name = "SETTING",
        number_of_values = 1,
        verbatim_doc_comment
    )]
    pub cranelift_enable: Vec<String>,

    /// Maximum size in bytes of wasm memory before it becomes dynamically
    /// relocatable instead of up-front-reserved.
    #[clap(long, value_name = "MAXIMUM")]
    pub static_memory_maximum_size: Option<u64>,

    /// Force using a "static" style for all wasm memories
    #[clap(long)]
    pub static_memory_forced: bool,

    /// Byte size of the guard region after static memories are allocated
    #[clap(long, value_name = "SIZE")]
    pub static_memory_guard_size: Option<u64>,

    /// Byte size of the guard region after dynamic memories are allocated
    #[clap(long, value_name = "SIZE")]
    pub dynamic_memory_guard_size: Option<u64>,

    /// Bytes to reserve at the end of linear memory for growth for dynamic
    /// memories.
    #[clap(long, value_name = "SIZE")]
    pub dynamic_memory_reserved_for_growth: Option<u64>,

    /// Enable Cranelift's internal debug verifier (expensive)
    #[clap(long)]
    pub enable_cranelift_debug_verifier: bool,

    /// Enable Cranelift's internal NaN canonicalization
    #[clap(long)]
    pub enable_cranelift_nan_canonicalization: bool,

    /// Enable execution fuel with N units fuel, where execution will trap after
    /// running out of fuel.
    ///
    /// Most WebAssembly instructions consume 1 unit of fuel. Some instructions,
    /// such as `nop`, `drop`, `block`, and `loop`, consume 0 units, as any
    /// execution cost associated with them involves other instructions which do
    /// consume fuel.
    #[clap(long, value_name = "N")]
    pub fuel: Option<u64>,

    /// Executing wasm code will yield when a global epoch counter
    /// changes, allowing for async operation without blocking the
    /// executor.
    #[clap(long)]
    pub epoch_interruption: bool,

    /// Disable the on-by-default address map from native code to wasm code
    #[clap(long)]
    pub disable_address_map: bool,

    /// Disable the default of attempting to initialize linear memory via a
    /// copy-on-write mapping
    #[clap(long)]
    pub disable_memory_init_cow: bool,

    /// Enable the pooling allocator, in place of the on-demand
    /// allocator.
    #[cfg(feature = "pooling-allocator")]
    #[clap(long)]
    pub pooling_allocator: bool,

    /// Maximum stack size, in bytes, that wasm is allowed to consume before a
    /// stack overflow is reported.
    #[clap(long)]
    pub max_wasm_stack: Option<usize>,

    /// Whether or not to force deterministic and host-independent behavior of
    /// the relaxed-simd instructions.
    ///
    /// By default these instructions may have architecture-specific behavior as
    /// allowed by the specification, but this can be used to force the behavior
    /// of these instructions to match the deterministic behavior classified in
    /// the specification. Note that enabling this option may come at a
    /// performance cost.
    #[clap(long)]
    pub relaxed_simd_deterministic: bool,
    /// Explicitly specify the name of the compiler to use for WebAssembly.
    ///
    /// Currently only `cranelift` and `winch` are supported, but not all builds
    /// of Wasmtime have both built in.
    #[clap(long)]
    pub compiler: Option<String>,
}

impl CommonOptions {
    pub fn init_logging(&self) {
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

    pub fn config(&self, target: Option<&str>) -> Result<Config> {
        let mut config = Config::new();

        config.strategy(match self.compiler.as_deref() {
            None => Strategy::Auto,
            Some("cranelift") => Strategy::Cranelift,
            Some("winch") => Strategy::Winch,
            Some(s) => bail!("unknown compiler: {s}"),
        });

        // Set the target before setting any cranelift options, since the
        // target will reset any target-specific options.
        if let Some(target) = target {
            config.target(target)?;
        }

        config
            .cranelift_debug_verifier(self.enable_cranelift_debug_verifier)
            .debug_info(self.debug_info)
            .cranelift_opt_level(self.opt_level())
            .cranelift_nan_canonicalization(self.enable_cranelift_nan_canonicalization);

        self.enable_wasm_features(&mut config);

        for name in &self.cranelift_enable {
            unsafe {
                config.cranelift_flag_enable(name);
            }
        }

        for (name, value) in &self.cranelift_set {
            unsafe {
                config.cranelift_flag_set(name, value);
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

        if self.disable_parallel_compilation {
            config.parallel_compilation(false);
        }

        if let Some(max) = self.static_memory_maximum_size {
            config.static_memory_maximum_size(max);
        }

        config.static_memory_forced(self.static_memory_forced);

        if let Some(size) = self.static_memory_guard_size {
            config.static_memory_guard_size(size);
        }

        if let Some(size) = self.dynamic_memory_guard_size {
            config.dynamic_memory_guard_size(size);
        }
        if let Some(size) = self.dynamic_memory_reserved_for_growth {
            config.dynamic_memory_reserved_for_growth(size);
        }

        // If fuel has been configured, set the `consume fuel` flag on the config.
        if self.fuel.is_some() {
            config.consume_fuel(true);
        }

        config.epoch_interruption(self.epoch_interruption);
        config.generate_address_map(!self.disable_address_map);
        config.memory_init_cow(!self.disable_memory_init_cow);

        #[cfg(feature = "pooling-allocator")]
        {
            if self.pooling_allocator {
                config.allocation_strategy(wasmtime::InstanceAllocationStrategy::pooling());
            }
        }

        if let Some(max) = self.max_wasm_stack {
            config.max_wasm_stack(max);
        }

        config.relaxed_simd_deterministic(self.relaxed_simd_deterministic);

        Ok(config)
    }

    pub fn enable_wasm_features(&self, config: &mut Config) {
        let WasmFeatures {
            simd,
            relaxed_simd,
            bulk_memory,
            reference_types,
            multi_value,
            tail_call,
            threads,
            multi_memory,
            memory64,
            #[cfg(feature = "component-model")]
            component_model,
            function_references,
        } = self.wasm_features.unwrap_or_default();

        if let Some(enable) = simd {
            config.wasm_simd(enable);
        }
        if let Some(enable) = relaxed_simd {
            config.wasm_relaxed_simd(enable);
        }
        if let Some(enable) = bulk_memory {
            config.wasm_bulk_memory(enable);
        }
        if let Some(enable) = reference_types {
            config.wasm_reference_types(enable);
        }
        if let Some(enable) = function_references {
            config.wasm_function_references(enable);
        }
        if let Some(enable) = multi_value {
            config.wasm_multi_value(enable);
        }
        if let Some(enable) = tail_call {
            config.wasm_tail_call(enable);
        }
        if let Some(enable) = threads {
            config.wasm_threads(enable);
        }
        if let Some(enable) = multi_memory {
            config.wasm_multi_memory(enable);
        }
        if let Some(enable) = memory64 {
            config.wasm_memory64(enable);
        }
        #[cfg(feature = "component-model")]
        if let Some(enable) = component_model {
            config.wasm_component_model(enable);
        }
    }

    pub fn opt_level(&self) -> wasmtime::OptLevel {
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

#[derive(Default, Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct WasmFeatures {
    pub reference_types: Option<bool>,
    pub multi_value: Option<bool>,
    pub bulk_memory: Option<bool>,
    pub simd: Option<bool>,
    pub relaxed_simd: Option<bool>,
    pub tail_call: Option<bool>,
    pub threads: Option<bool>,
    pub multi_memory: Option<bool>,
    pub memory64: Option<bool>,
    #[cfg(feature = "component-model")]
    pub component_model: Option<bool>,
    pub function_references: Option<bool>,
}

fn parse_wasm_features(features: &str) -> Result<WasmFeatures> {
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

    Ok(WasmFeatures {
        reference_types: all.or(values["reference-types"]),
        multi_value: all.or(values["multi-value"]),
        bulk_memory: all.or(values["bulk-memory"]),
        simd: all.or(values["simd"]),
        relaxed_simd: all.or(values["relaxed-simd"]),
        tail_call: all.or(values["tail-call"]),
        threads: all.or(values["threads"]),
        multi_memory: all.or(values["multi-memory"]),
        memory64: all.or(values["memory64"]),
        #[cfg(feature = "component-model")]
        component_model: all.or(values["component-model"]),
        function_references: all.or(values["function-references"]),
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
                "experimental-wasi-threads" => Ok(wasi_modules.wasi_threads = enable),
                "experimental-wasi-http" => Ok(wasi_modules.wasi_http = enable),
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

    /// Enable the experimental wasi-threads implementation.
    pub wasi_threads: bool,

    /// Enable the experimental wasi-http implementation
    pub wasi_http: bool,
}

impl Default for WasiModules {
    fn default() -> Self {
        Self {
            wasi_common: true,
            wasi_nn: false,
            wasi_threads: false,
            wasi_http: false,
        }
    }
}

impl WasiModules {
    /// Enable no modules.
    pub fn none() -> Self {
        Self {
            wasi_common: false,
            wasi_nn: false,
            wasi_threads: false,
            wasi_http: false,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_all_features() -> Result<()> {
        let options = CommonOptions::try_parse_from(vec!["foo", "--wasm-features=all"])?;

        let WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            simd,
            relaxed_simd,
            tail_call,
            threads,
            multi_memory,
            memory64,
            function_references,
            #[cfg(feature = "component-model")]
            component_model,
        } = options.wasm_features.unwrap();

        assert_eq!(reference_types, Some(true));
        assert_eq!(multi_value, Some(true));
        assert_eq!(bulk_memory, Some(true));
        assert_eq!(simd, Some(true));
        assert_eq!(tail_call, Some(true));
        assert_eq!(threads, Some(true));
        assert_eq!(multi_memory, Some(true));
        assert_eq!(memory64, Some(true));
        assert_eq!(function_references, Some(true));
        assert_eq!(relaxed_simd, Some(true));
        #[cfg(feature = "component-model")]
        assert_eq!(component_model, Some(true));

        Ok(())
    }

    #[test]
    fn test_no_features() -> Result<()> {
        let options = CommonOptions::try_parse_from(vec!["foo", "--wasm-features=-all"])?;

        let WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            simd,
            relaxed_simd,
            tail_call,
            threads,
            multi_memory,
            memory64,
            function_references,
            #[cfg(feature = "component-model")]
            component_model,
        } = options.wasm_features.unwrap();

        assert_eq!(reference_types, Some(false));
        assert_eq!(multi_value, Some(false));
        assert_eq!(bulk_memory, Some(false));
        assert_eq!(simd, Some(false));
        assert_eq!(tail_call, Some(false));
        assert_eq!(threads, Some(false));
        assert_eq!(multi_memory, Some(false));
        assert_eq!(memory64, Some(false));
        assert_eq!(function_references, Some(false));
        assert_eq!(relaxed_simd, Some(false));
        #[cfg(feature = "component-model")]
        assert_eq!(component_model, Some(false));

        Ok(())
    }

    #[test]
    fn test_multiple_features() -> Result<()> {
        let options = CommonOptions::try_parse_from(vec![
            "foo",
            "--wasm-features=-reference-types,simd,multi-memory,memory64",
        ])?;

        let WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            simd,
            relaxed_simd,
            tail_call,
            threads,
            multi_memory,
            memory64,
            function_references,
            #[cfg(feature = "component-model")]
            component_model,
        } = options.wasm_features.unwrap();

        assert_eq!(reference_types, Some(false));
        assert_eq!(multi_value, None);
        assert_eq!(bulk_memory, None);
        assert_eq!(simd, Some(true));
        assert_eq!(tail_call, None);
        assert_eq!(threads, None);
        assert_eq!(multi_memory, Some(true));
        assert_eq!(memory64, Some(true));
        assert_eq!(function_references, None);
        assert_eq!(relaxed_simd, None);
        #[cfg(feature = "component-model")]
        assert_eq!(component_model, None);

        Ok(())
    }

    macro_rules! feature_test {
        ($test_name:ident, $name:ident, $flag:literal) => {
            #[test]
            fn $test_name() -> Result<()> {
                let options =
                    CommonOptions::try_parse_from(vec!["foo", concat!("--wasm-features=", $flag)])?;

                let WasmFeatures { $name, .. } = options.wasm_features.unwrap();

                assert_eq!($name, Some(true));

                let options = CommonOptions::try_parse_from(vec![
                    "foo",
                    concat!("--wasm-features=-", $flag),
                ])?;

                let WasmFeatures { $name, .. } = options.wasm_features.unwrap();

                assert_eq!($name, Some(false));

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
    feature_test!(test_simd_feature, simd, "simd");
    feature_test!(test_relaxed_simd_feature, relaxed_simd, "relaxed-simd");
    feature_test!(test_tail_call_feature, tail_call, "tail-call");
    feature_test!(test_threads_feature, threads, "threads");
    feature_test!(test_multi_memory_feature, multi_memory, "multi-memory");
    feature_test!(test_memory64_feature, memory64, "memory64");

    #[test]
    fn test_default_modules() {
        let options = CommonOptions::try_parse_from(vec!["foo", "--wasi-modules=default"]).unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: true,
                wasi_nn: false,
                wasi_threads: false,
                wasi_http: false,
            }
        );
    }

    #[test]
    fn test_empty_modules() {
        let options = CommonOptions::try_parse_from(vec!["foo", "--wasi-modules="]).unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: true,
                wasi_nn: false,
                wasi_threads: false,
                wasi_http: false
            }
        );
    }

    #[test]
    fn test_some_modules() {
        let options = CommonOptions::try_parse_from(vec![
            "foo",
            "--wasi-modules=experimental-wasi-nn,-wasi-common",
        ])
        .unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: false,
                wasi_nn: true,
                wasi_threads: false,
                wasi_http: false,
            }
        );
    }

    #[test]
    fn test_no_modules() {
        let options =
            CommonOptions::try_parse_from(vec!["foo", "--wasi-modules=-default"]).unwrap();
        assert_eq!(
            options.wasi_modules.unwrap(),
            WasiModules {
                wasi_common: false,
                wasi_nn: false,
                wasi_threads: false,
                wasi_http: false,
            }
        );
    }
}
