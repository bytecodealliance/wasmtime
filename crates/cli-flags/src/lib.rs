//! Contains the common Wasmtime command line interface (CLI) flags.

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    time::Duration,
};
use wasmtime::Config;

pub mod opt;

#[cfg(feature = "logging")]
fn init_file_per_thread_logger(prefix: &'static str) {
    file_per_thread_logger::initialize(prefix);
    file_per_thread_logger::allow_uninitialized();

    // Extending behavior of default spawner:
    // https://docs.rs/rayon/1.1.0/rayon/struct.ThreadPoolBuilder.html#method.spawn_handler
    // Source code says DefaultSpawner is implementation detail and
    // shouldn't be used directly.
    #[cfg(feature = "parallel-compilation")]
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

wasmtime_option_group! {
    #[derive(PartialEq, Clone, Deserialize)]
    #[serde(rename_all = "kebab-case", deny_unknown_fields)]
    pub struct OptimizeOptions {
        /// Optimization level of generated code (0-2, s; default: 2)
        #[serde(default)]
        #[serde(deserialize_with = "crate::opt::cli_parse_wrapper")]
        pub opt_level: Option<wasmtime::OptLevel>,

        /// Register allocator algorithm choice.
        #[serde(default)]
        #[serde(deserialize_with = "crate::opt::cli_parse_wrapper")]
        pub regalloc_algorithm: Option<wasmtime::RegallocAlgorithm>,

        /// Do not allow Wasm linear memories to move in the host process's
        /// address space.
        pub memory_may_move: Option<bool>,

        /// Initial virtual memory allocation size for memories.
        pub memory_reservation: Option<u64>,

        /// Bytes to reserve at the end of linear memory for growth into.
        pub memory_reservation_for_growth: Option<u64>,

        /// Size, in bytes, of guard pages for linear memories.
        pub memory_guard_size: Option<u64>,

        /// Indicates whether an unmapped region of memory is placed before all
        /// linear memories.
        pub guard_before_linear_memory: Option<bool>,

        /// Whether to initialize tables lazily, so that instantiation is
        /// fast but indirect calls are a little slower. If no, tables are
        /// initialized eagerly from any active element segments that apply to
        /// them during instantiation. (default: yes)
        pub table_lazy_init: Option<bool>,

        /// Enable the pooling allocator, in place of the on-demand allocator.
        pub pooling_allocator: Option<bool>,

        /// The number of decommits to do per batch. A batch size of 1
        /// effectively disables decommit batching. (default: 1)
        pub pooling_decommit_batch_size: Option<usize>,

        /// How many bytes to keep resident between instantiations for the
        /// pooling allocator in linear memories.
        pub pooling_memory_keep_resident: Option<usize>,

        /// How many bytes to keep resident between instantiations for the
        /// pooling allocator in tables.
        pub pooling_table_keep_resident: Option<usize>,

        /// Enable memory protection keys for the pooling allocator; this can
        /// optimize the size of memory slots.
        #[serde(default)]
        #[serde(deserialize_with = "crate::opt::cli_parse_wrapper")]
        pub pooling_memory_protection_keys: Option<wasmtime::MpkEnabled>,

        /// Sets an upper limit on how many memory protection keys (MPK) Wasmtime
        /// will use. (default: 16)
        pub pooling_max_memory_protection_keys: Option<usize>,

        /// Configure attempting to initialize linear memory via a
        /// copy-on-write mapping (default: yes)
        pub memory_init_cow: Option<bool>,

        /// Threshold below which CoW images are guaranteed to be used and be
        /// dense.
        pub memory_guaranteed_dense_image_size: Option<u64>,

        /// The maximum number of WebAssembly instances which can be created
        /// with the pooling allocator.
        pub pooling_total_core_instances: Option<u32>,

        /// The maximum number of WebAssembly components which can be created
        /// with the pooling allocator.
        pub pooling_total_component_instances: Option<u32>,

        /// The maximum number of WebAssembly memories which can be created with
        /// the pooling allocator.
        pub pooling_total_memories: Option<u32>,

        /// The maximum number of WebAssembly tables which can be created with
        /// the pooling allocator.
        pub pooling_total_tables: Option<u32>,

        /// The maximum number of WebAssembly stacks which can be created with
        /// the pooling allocator.
        pub pooling_total_stacks: Option<u32>,

        /// The maximum runtime size of each linear memory in the pooling
        /// allocator, in bytes.
        pub pooling_max_memory_size: Option<usize>,

        /// The maximum table elements for any table defined in a module when
        /// using the pooling allocator.
        pub pooling_table_elements: Option<usize>,

        /// The maximum size, in bytes, allocated for a core instance's metadata
        /// when using the pooling allocator.
        pub pooling_max_core_instance_size: Option<usize>,

        /// Configures the maximum number of "unused warm slots" to retain in the
        /// pooling allocator. (default: 100)
        pub pooling_max_unused_warm_slots: Option<u32>,

        /// How much memory, in bytes, to keep resident for async stacks allocated
        /// with the pooling allocator. (default: 0)
        pub pooling_async_stack_keep_resident: Option<usize>,

        /// The maximum size, in bytes, allocated for a component instance's
        /// `VMComponentContext` metadata. (default: 1MiB)
        pub pooling_max_component_instance_size: Option<usize>,

        /// The maximum number of core instances a single component may contain
        /// (default is unlimited).
        pub pooling_max_core_instances_per_component: Option<u32>,

        /// The maximum number of Wasm linear memories that a single component may
        /// transitively contain (default is unlimited).
        pub pooling_max_memories_per_component: Option<u32>,

        /// The maximum number of tables that a single component may transitively
        /// contain (default is unlimited).
        pub pooling_max_tables_per_component: Option<u32>,

        /// The maximum number of defined tables for a core module. (default: 1)
        pub pooling_max_tables_per_module: Option<u32>,

        /// The maximum number of defined linear memories for a module. (default: 1)
        pub pooling_max_memories_per_module: Option<u32>,

        /// The maximum number of concurrent GC heaps supported. (default: 1000)
        pub pooling_total_gc_heaps: Option<u32>,

        /// Enable or disable the use of host signal handlers for traps.
        pub signals_based_traps: Option<bool>,

        /// DEPRECATED: Use `-Cmemory-guard-size=N` instead.
        pub dynamic_memory_guard_size: Option<u64>,

        /// DEPRECATED: Use `-Cmemory-guard-size=N` instead.
        pub static_memory_guard_size: Option<u64>,

        /// DEPRECATED: Use `-Cmemory-may-move` instead.
        pub static_memory_forced: Option<bool>,

        /// DEPRECATED: Use `-Cmemory-reservation=N` instead.
        pub static_memory_maximum_size: Option<u64>,

        /// DEPRECATED: Use `-Cmemory-reservation-for-growth=N` instead.
        pub dynamic_memory_reserved_for_growth: Option<u64>,
    }

    enum Optimize {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone, Deserialize)]
    #[serde(rename_all = "kebab-case", deny_unknown_fields)]
    pub struct CodegenOptions {
        /// Either `cranelift` or `winch`.
        ///
        /// Currently only `cranelift` and `winch` are supported, but not all
        /// builds of Wasmtime have both built in.
        #[serde(default)]
        #[serde(deserialize_with = "crate::opt::cli_parse_wrapper")]
        pub compiler: Option<wasmtime::Strategy>,
        /// Which garbage collector to use: `drc` or `null`.
        ///
        /// `drc` is the deferred reference-counting collector.
        ///
        /// `null` is the null garbage collector, which does not collect any
        /// garbage.
        ///
        /// Note that not all builds of Wasmtime will have support for garbage
        /// collection included.
        #[serde(default)]
        #[serde(deserialize_with = "crate::opt::cli_parse_wrapper")]
        pub collector: Option<wasmtime::Collector>,
        /// Enable Cranelift's internal debug verifier (expensive)
        pub cranelift_debug_verifier: Option<bool>,
        /// Whether or not to enable caching of compiled modules.
        pub cache: Option<bool>,
        /// Configuration for compiled module caching.
        pub cache_config: Option<String>,
        /// Whether or not to enable parallel compilation of modules.
        pub parallel_compilation: Option<bool>,
        /// Whether to enable proof-carrying code (PCC)-based validation.
        pub pcc: Option<bool>,
        /// Controls whether native unwind information is present in compiled
        /// object files.
        pub native_unwind_info: Option<bool>,

        #[prefixed = "cranelift"]
        #[serde(default)]
        /// Set a cranelift-specific option. Use `wasmtime settings` to see
        /// all.
        pub cranelift: Vec<(String, Option<String>)>,
    }

    enum Codegen {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone, Deserialize)]
    #[serde(rename_all = "kebab-case", deny_unknown_fields)]
    pub struct DebugOptions {
        /// Enable generation of DWARF debug information in compiled code.
        pub debug_info: Option<bool>,
        /// Configure whether compiled code can map native addresses to wasm.
        pub address_map: Option<bool>,
        /// Configure whether logging is enabled.
        pub logging: Option<bool>,
        /// Configure whether logs are emitted to files
        pub log_to_files: Option<bool>,
        /// Enable coredump generation to this file after a WebAssembly trap.
        pub coredump: Option<String>,
    }

    enum Debug {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone, Deserialize)]
    #[serde(rename_all = "kebab-case", deny_unknown_fields)]
    pub struct WasmOptions {
        /// Enable canonicalization of all NaN values.
        pub nan_canonicalization: Option<bool>,
        /// Enable execution fuel with N units fuel, trapping after running out
        /// of fuel.
        ///
        /// Most WebAssembly instructions consume 1 unit of fuel. Some
        /// instructions, such as `nop`, `drop`, `block`, and `loop`, consume 0
        /// units, as any execution cost associated with them involves other
        /// instructions which do consume fuel.
        pub fuel: Option<u64>,
        /// Yield when a global epoch counter changes, allowing for async
        /// operation without blocking the executor.
        pub epoch_interruption: Option<bool>,
        /// Maximum stack size, in bytes, that wasm is allowed to consume before a
        /// stack overflow is reported.
        pub max_wasm_stack: Option<usize>,
        /// Stack size, in bytes, that will be allocated for async stacks.
        ///
        /// Note that this must be larger than `max-wasm-stack` and the
        /// difference between the two is how much stack the host has to execute
        /// on.
        pub async_stack_size: Option<usize>,
        /// Configures whether or not stacks used for async futures are zeroed
        /// before (re)use as a defense-in-depth mechanism. (default: false)
        pub async_stack_zeroing: Option<bool>,
        /// Allow unknown exports when running commands.
        pub unknown_exports_allow: Option<bool>,
        /// Allow the main module to import unknown functions, using an
        /// implementation that immediately traps, when running commands.
        pub unknown_imports_trap: Option<bool>,
        /// Allow the main module to import unknown functions, using an
        /// implementation that returns default values, when running commands.
        pub unknown_imports_default: Option<bool>,
        /// Enables memory error checking. (see wmemcheck.md for more info)
        pub wmemcheck: Option<bool>,
        /// Maximum size, in bytes, that a linear memory is allowed to reach.
        ///
        /// Growth beyond this limit will cause `memory.grow` instructions in
        /// WebAssembly modules to return -1 and fail.
        pub max_memory_size: Option<usize>,
        /// Maximum size, in table elements, that a table is allowed to reach.
        pub max_table_elements: Option<usize>,
        /// Maximum number of WebAssembly instances allowed to be created.
        pub max_instances: Option<usize>,
        /// Maximum number of WebAssembly tables allowed to be created.
        pub max_tables: Option<usize>,
        /// Maximum number of WebAssembly linear memories allowed to be created.
        pub max_memories: Option<usize>,
        /// Force a trap to be raised on `memory.grow` and `table.grow` failure
        /// instead of returning -1 from these instructions.
        ///
        /// This is not necessarily a spec-compliant option to enable but can be
        /// useful for tracking down a backtrace of what is requesting so much
        /// memory, for example.
        pub trap_on_grow_failure: Option<bool>,
        /// Maximum execution time of wasm code before timing out (1, 2s, 100ms, etc)
        pub timeout: Option<Duration>,
        /// Configures support for all WebAssembly proposals implemented.
        pub all_proposals: Option<bool>,
        /// Configure support for the bulk memory proposal.
        pub bulk_memory: Option<bool>,
        /// Configure support for the multi-memory proposal.
        pub multi_memory: Option<bool>,
        /// Configure support for the multi-value proposal.
        pub multi_value: Option<bool>,
        /// Configure support for the reference-types proposal.
        pub reference_types: Option<bool>,
        /// Configure support for the simd proposal.
        pub simd: Option<bool>,
        /// Configure support for the relaxed-simd proposal.
        pub relaxed_simd: Option<bool>,
        /// Configure forcing deterministic and host-independent behavior of
        /// the relaxed-simd instructions.
        ///
        /// By default these instructions may have architecture-specific behavior as
        /// allowed by the specification, but this can be used to force the behavior
        /// of these instructions to match the deterministic behavior classified in
        /// the specification. Note that enabling this option may come at a
        /// performance cost.
        pub relaxed_simd_deterministic: Option<bool>,
        /// Configure support for the tail-call proposal.
        pub tail_call: Option<bool>,
        /// Configure support for the threads proposal.
        pub threads: Option<bool>,
        /// Configure support for the memory64 proposal.
        pub memory64: Option<bool>,
        /// Configure support for the component-model proposal.
        pub component_model: Option<bool>,
        /// Configure support for 33+ flags in the component model.
        pub component_model_more_flags: Option<bool>,
        /// Component model support for more than one return value.
        pub component_model_multiple_returns: Option<bool>,
        /// Component model support for async lifting/lowering.
        pub component_model_async: Option<bool>,
        /// Configure support for the function-references proposal.
        pub function_references: Option<bool>,
        /// Configure support for the GC proposal.
        pub gc: Option<bool>,
        /// Configure support for the custom-page-sizes proposal.
        pub custom_page_sizes: Option<bool>,
        /// Configure support for the wide-arithmetic proposal.
        pub wide_arithmetic: Option<bool>,
        /// Configure support for the extended-const proposal.
        pub extended_const: Option<bool>,
    }

    enum Wasm {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone, Deserialize)]
    #[serde(rename_all = "kebab-case", deny_unknown_fields)]
    pub struct WasiOptions {
        /// Enable support for WASI CLI APIs, including filesystems, sockets, clocks, and random.
        pub cli: Option<bool>,
        /// Enable WASI APIs marked as: @unstable(feature = cli-exit-with-code)
        pub cli_exit_with_code: Option<bool>,
        /// Deprecated alias for `cli`
        pub common: Option<bool>,
        /// Enable support for WASI neural network imports (experimental)
        pub nn: Option<bool>,
        /// Enable support for WASI threading imports (experimental). Implies preview2=false.
        pub threads: Option<bool>,
        /// Enable support for WASI HTTP imports
        pub http: Option<bool>,
        /// Number of distinct write calls to the outgoing body's output-stream
        /// that the implementation will buffer.
        /// Default: 1.
        pub http_outgoing_body_buffer_chunks: Option<usize>,
        /// Maximum size allowed in a write call to the outgoing body's output-stream.
        /// Default: 1024 * 1024.
        pub http_outgoing_body_chunk_size: Option<usize>,
        /// Enable support for WASI config imports (experimental)
        pub config: Option<bool>,
        /// Enable support for WASI key-value imports (experimental)
        pub keyvalue: Option<bool>,
        /// Inherit environment variables and file descriptors following the
        /// systemd listen fd specification (UNIX only)
        pub listenfd: Option<bool>,
        /// Grant access to the given TCP listen socket
        #[serde(default)]
        pub tcplisten: Vec<String>,
        /// Implement WASI Preview1 using new Preview2 implementation (true, default) or legacy
        /// implementation (false)
        pub preview2: Option<bool>,
        /// Pre-load machine learning graphs (i.e., models) for use by wasi-nn.
        ///
        /// Each use of the flag will preload a ML model from the host directory
        /// using the given model encoding. The model will be mapped to the
        /// directory name: e.g., `--wasi-nn-graph openvino:/foo/bar` will preload
        /// an OpenVINO model named `bar`. Note that which model encodings are
        /// available is dependent on the backends implemented in the
        /// `wasmtime_wasi_nn` crate.
        #[serde(skip)]
        pub nn_graph: Vec<WasiNnGraph>,
        /// Flag for WASI preview2 to inherit the host's network within the
        /// guest so it has full access to all addresses/ports/etc.
        pub inherit_network: Option<bool>,
        /// Indicates whether `wasi:sockets/ip-name-lookup` is enabled or not.
        pub allow_ip_name_lookup: Option<bool>,
        /// Indicates whether `wasi:sockets` TCP support is enabled or not.
        pub tcp: Option<bool>,
        /// Indicates whether `wasi:sockets` UDP support is enabled or not.
        pub udp: Option<bool>,
        /// Enable WASI APIs marked as: @unstable(feature = network-error-code)
        pub network_error_code: Option<bool>,
        /// Allows imports from the `wasi_unstable` core wasm module.
        pub preview0: Option<bool>,
        /// Inherit all environment variables from the parent process.
        ///
        /// This option can be further overwritten with `--env` flags.
        pub inherit_env: Option<bool>,
        /// Pass a wasi config variable to the program.
        #[serde(skip)]
        pub config_var: Vec<KeyValuePair>,
        /// Preset data for the In-Memory provider of WASI key-value API.
        #[serde(skip)]
        pub keyvalue_in_memory_data: Vec<KeyValuePair>,
    }

    enum Wasi {
        ...
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WasiNnGraph {
    pub format: String,
    pub dir: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

/// Common options for commands that translate WebAssembly modules
#[derive(Parser, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonOptions {
    // These options groups are used to parse `-O` and such options but aren't
    // the raw form consumed by the CLI. Instead they're pushed into the `pub`
    // fields below as part of the `configure` method.
    //
    // Ideally clap would support `pub opts: OptimizeOptions` and parse directly
    // into that but it does not appear to do so for multiple `-O` flags for
    // now.
    /// Optimization and tuning related options for wasm performance, `-O help` to
    /// see all.
    #[arg(short = 'O', long = "optimize", value_name = "KEY[=VAL[,..]]")]
    #[serde(skip)]
    opts_raw: Vec<opt::CommaSeparated<Optimize>>,

    /// Codegen-related configuration options, `-C help` to see all.
    #[arg(short = 'C', long = "codegen", value_name = "KEY[=VAL[,..]]")]
    #[serde(skip)]
    codegen_raw: Vec<opt::CommaSeparated<Codegen>>,

    /// Debug-related configuration options, `-D help` to see all.
    #[arg(short = 'D', long = "debug", value_name = "KEY[=VAL[,..]]")]
    #[serde(skip)]
    debug_raw: Vec<opt::CommaSeparated<Debug>>,

    /// Options for configuring semantic execution of WebAssembly, `-W help` to see
    /// all.
    #[arg(short = 'W', long = "wasm", value_name = "KEY[=VAL[,..]]")]
    #[serde(skip)]
    wasm_raw: Vec<opt::CommaSeparated<Wasm>>,

    /// Options for configuring WASI and its proposals, `-S help` to see all.
    #[arg(short = 'S', long = "wasi", value_name = "KEY[=VAL[,..]]")]
    #[serde(skip)]
    wasi_raw: Vec<opt::CommaSeparated<Wasi>>,

    // These fields are filled in by the `configure` method below via the
    // options parsed from the CLI above. This is what the CLI should use.
    #[arg(skip)]
    #[serde(skip)]
    configured: bool,

    #[arg(skip)]
    #[serde(rename = "optimize", default)]
    pub opts: OptimizeOptions,

    #[arg(skip)]
    #[serde(rename = "codegen", default)]
    pub codegen: CodegenOptions,

    #[arg(skip)]
    #[serde(rename = "debug", default)]
    pub debug: DebugOptions,

    #[arg(skip)]
    #[serde(rename = "wasm", default)]
    pub wasm: WasmOptions,

    #[arg(skip)]
    #[serde(rename = "wasi", default)]
    pub wasi: WasiOptions,

    /// The target triple; default is the host triple
    #[arg(long, value_name = "TARGET")]
    #[serde(skip)]
    pub target: Option<String>,

    /// Use the specified TOML configuration file.
    /// This TOML configuration file can provide same configuration options as the
    /// `--optimize`, `--codgen`, `--debug`, `--wasm`, `--wasi` CLI options, with a couple exceptions.
    ///
    /// Additional options specified on the command line will take precedent over options loaded from
    /// this TOML file.
    #[arg(long = "config", value_name = "FILE")]
    #[serde(skip)]
    pub config: Option<PathBuf>,
}

macro_rules! match_feature {
    (
        [$feat:tt : $config:expr]
        $val:ident => $e:expr,
        $p:pat => err,
    ) => {
        #[cfg(feature = $feat)]
        {
            if let Some($val) = $config {
                $e;
            }
        }
        #[cfg(not(feature = $feat))]
        {
            if let Some($p) = $config {
                anyhow::bail!(concat!("support for ", $feat, " disabled at compile time"));
            }
        }
    };
}

impl CommonOptions {
    /// Creates a blank new set of [`CommonOptions`] that can be configured.
    pub fn new() -> CommonOptions {
        CommonOptions {
            opts_raw: Vec::new(),
            codegen_raw: Vec::new(),
            debug_raw: Vec::new(),
            wasm_raw: Vec::new(),
            wasi_raw: Vec::new(),
            configured: true,
            opts: Default::default(),
            codegen: Default::default(),
            debug: Default::default(),
            wasm: Default::default(),
            wasi: Default::default(),
            target: None,
            config: None,
        }
    }

    fn configure(&mut self) -> Result<()> {
        if self.configured {
            return Ok(());
        }
        self.configured = true;
        if let Some(toml_config_path) = &self.config {
            let toml_options = CommonOptions::from_file(toml_config_path)?;
            self.opts = toml_options.opts;
            self.codegen = toml_options.codegen;
            self.debug = toml_options.debug;
            self.wasm = toml_options.wasm;
            self.wasi = toml_options.wasi;
        }
        self.opts.configure_with(&self.opts_raw);
        self.codegen.configure_with(&self.codegen_raw);
        self.debug.configure_with(&self.debug_raw);
        self.wasm.configure_with(&self.wasm_raw);
        self.wasi.configure_with(&self.wasi_raw);
        Ok(())
    }

    pub fn init_logging(&mut self) -> Result<()> {
        self.configure()?;
        if self.debug.logging == Some(false) {
            return Ok(());
        }
        #[cfg(feature = "logging")]
        if self.debug.log_to_files == Some(true) {
            let prefix = "wasmtime.dbg.";
            init_file_per_thread_logger(prefix);
        } else {
            use std::io::IsTerminal;
            use tracing_subscriber::{EnvFilter, FmtSubscriber};
            let builder = FmtSubscriber::builder()
                .with_writer(std::io::stderr)
                .with_env_filter(EnvFilter::from_env("WASMTIME_LOG"))
                .with_ansi(std::io::stderr().is_terminal());
            if std::env::var("WASMTIME_LOG_NO_CONTEXT").is_ok_and(|value| value.eq("1")) {
                builder
                    .with_level(false)
                    .with_target(false)
                    .without_time()
                    .init()
            } else {
                builder.init();
            }
        }
        #[cfg(not(feature = "logging"))]
        if self.debug.log_to_files == Some(true) || self.debug.logging == Some(true) {
            anyhow::bail!("support for logging disabled at compile time");
        }
        Ok(())
    }

    pub fn config(&mut self, pooling_allocator_default: Option<bool>) -> Result<Config> {
        self.configure()?;
        let mut config = Config::new();

        match_feature! {
            ["cranelift" : self.codegen.compiler]
            strategy => config.strategy(strategy),
            _ => err,
        }
        match_feature! {
            ["gc" : self.codegen.collector]
            collector => config.collector(collector),
            _ => err,
        }
        if let Some(target) = &self.target {
            config.target(target)?;
        }
        match_feature! {
            ["cranelift" : self.codegen.cranelift_debug_verifier]
            enable => config.cranelift_debug_verifier(enable),
            true => err,
        }
        if let Some(enable) = self.debug.debug_info {
            config.debug_info(enable);
        }
        if self.debug.coredump.is_some() {
            #[cfg(feature = "coredump")]
            config.coredump_on_trap(true);
            #[cfg(not(feature = "coredump"))]
            anyhow::bail!("support for coredumps disabled at compile time");
        }
        match_feature! {
            ["cranelift" : self.opts.opt_level]
            level => config.cranelift_opt_level(level),
            _ => err,
        }
        match_feature! {
            ["cranelift": self.opts.regalloc_algorithm]
            algo => config.cranelift_regalloc_algorithm(algo),
            _ => err,
        }
        match_feature! {
            ["cranelift" : self.wasm.nan_canonicalization]
            enable => config.cranelift_nan_canonicalization(enable),
            true => err,
        }
        match_feature! {
            ["cranelift" : self.codegen.pcc]
            enable => config.cranelift_pcc(enable),
            true => err,
        }

        self.enable_wasm_features(&mut config)?;

        #[cfg(feature = "cranelift")]
        for (name, value) in self.codegen.cranelift.iter() {
            let name = name.replace('-', "_");
            unsafe {
                match value {
                    Some(val) => {
                        config.cranelift_flag_set(&name, val);
                    }
                    None => {
                        config.cranelift_flag_enable(&name);
                    }
                }
            }
        }
        #[cfg(not(feature = "cranelift"))]
        if !self.codegen.cranelift.is_empty() {
            anyhow::bail!("support for cranelift disabled at compile time");
        }

        #[cfg(feature = "cache")]
        if self.codegen.cache != Some(false) {
            match &self.codegen.cache_config {
                Some(path) => {
                    config.cache_config_load(path)?;
                }
                None => {
                    config.cache_config_load_default()?;
                }
            }
        }
        #[cfg(not(feature = "cache"))]
        if self.codegen.cache == Some(true) {
            anyhow::bail!("support for caching disabled at compile time");
        }

        match_feature! {
            ["parallel-compilation" : self.codegen.parallel_compilation]
            enable => config.parallel_compilation(enable),
            true => err,
        }

        let memory_reservation = self
            .opts
            .memory_reservation
            .or(self.opts.static_memory_maximum_size);
        if let Some(size) = memory_reservation {
            config.memory_reservation(size);
        }

        if let Some(enable) = self.opts.static_memory_forced {
            config.memory_may_move(!enable);
        }
        if let Some(enable) = self.opts.memory_may_move {
            config.memory_may_move(enable);
        }

        let memory_guard_size = self
            .opts
            .static_memory_guard_size
            .or(self.opts.dynamic_memory_guard_size)
            .or(self.opts.memory_guard_size);
        if let Some(size) = memory_guard_size {
            config.memory_guard_size(size);
        }

        let mem_for_growth = self
            .opts
            .memory_reservation_for_growth
            .or(self.opts.dynamic_memory_reserved_for_growth);
        if let Some(size) = mem_for_growth {
            config.memory_reservation_for_growth(size);
        }
        if let Some(enable) = self.opts.guard_before_linear_memory {
            config.guard_before_linear_memory(enable);
        }
        if let Some(enable) = self.opts.table_lazy_init {
            config.table_lazy_init(enable);
        }

        // If fuel has been configured, set the `consume fuel` flag on the config.
        if self.wasm.fuel.is_some() {
            config.consume_fuel(true);
        }

        if let Some(enable) = self.wasm.epoch_interruption {
            config.epoch_interruption(enable);
        }
        if let Some(enable) = self.debug.address_map {
            config.generate_address_map(enable);
        }
        if let Some(enable) = self.opts.memory_init_cow {
            config.memory_init_cow(enable);
        }
        if let Some(size) = self.opts.memory_guaranteed_dense_image_size {
            config.memory_guaranteed_dense_image_size(size);
        }
        if let Some(enable) = self.opts.signals_based_traps {
            config.signals_based_traps(enable);
        }
        if let Some(enable) = self.codegen.native_unwind_info {
            config.native_unwind_info(enable);
        }

        match_feature! {
            ["pooling-allocator" : self.opts.pooling_allocator.or(pooling_allocator_default)]
            enable => {
                if enable {
                    let mut cfg = wasmtime::PoolingAllocationConfig::default();
                    if let Some(size) = self.opts.pooling_memory_keep_resident {
                        cfg.linear_memory_keep_resident(size);
                    }
                    if let Some(size) = self.opts.pooling_table_keep_resident {
                        cfg.table_keep_resident(size);
                    }
                    if let Some(limit) = self.opts.pooling_total_core_instances {
                        cfg.total_core_instances(limit);
                    }
                    if let Some(limit) = self.opts.pooling_total_component_instances {
                        cfg.total_component_instances(limit);
                    }
                    if let Some(limit) = self.opts.pooling_total_memories {
                        cfg.total_memories(limit);
                    }
                    if let Some(limit) = self.opts.pooling_total_tables {
                        cfg.total_tables(limit);
                    }
                    if let Some(limit) = self.opts.pooling_table_elements {
                        cfg.table_elements(limit);
                    }
                    if let Some(limit) = self.opts.pooling_max_core_instance_size {
                        cfg.max_core_instance_size(limit);
                    }
                    match_feature! {
                        ["async" : self.opts.pooling_total_stacks]
                        limit => cfg.total_stacks(limit),
                        _ => err,
                    }
                    if let Some(max) = self.opts.pooling_max_memory_size {
                        cfg.max_memory_size(max);
                    }
                    if let Some(size) = self.opts.pooling_decommit_batch_size {
                        cfg.decommit_batch_size(size);
                    }
                    if let Some(max) = self.opts.pooling_max_unused_warm_slots {
                        cfg.max_unused_warm_slots(max);
                    }
                    match_feature! {
                        ["async" : self.opts.pooling_async_stack_keep_resident]
                        size => cfg.async_stack_keep_resident(size),
                        _ => err,
                    }
                    if let Some(max) = self.opts.pooling_max_component_instance_size {
                        cfg.max_component_instance_size(max);
                    }
                    if let Some(max) = self.opts.pooling_max_core_instances_per_component {
                        cfg.max_core_instances_per_component(max);
                    }
                    if let Some(max) = self.opts.pooling_max_memories_per_component {
                        cfg.max_memories_per_component(max);
                    }
                    if let Some(max) = self.opts.pooling_max_tables_per_component {
                        cfg.max_tables_per_component(max);
                    }
                    if let Some(max) = self.opts.pooling_max_tables_per_module {
                        cfg.max_tables_per_module(max);
                    }
                    if let Some(max) = self.opts.pooling_max_memories_per_module {
                        cfg.max_memories_per_module(max);
                    }
                    match_feature! {
                        ["memory-protection-keys" : self.opts.pooling_memory_protection_keys]
                        enable => cfg.memory_protection_keys(enable),
                        _ => err,
                    }
                    match_feature! {
                        ["memory-protection-keys" : self.opts.pooling_max_memory_protection_keys]
                        max => cfg.max_memory_protection_keys(max),
                        _ => err,
                    }
                    match_feature! {
                        ["gc" : self.opts.pooling_total_gc_heaps]
                        max => cfg.total_gc_heaps(max),
                        _ => err,
                    }
                    config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(cfg));
                }
            },
            true => err,
        }

        if self.opts.pooling_memory_protection_keys.is_some()
            && !self.opts.pooling_allocator.unwrap_or(false)
        {
            anyhow::bail!("memory protection keys require the pooling allocator");
        }

        if self.opts.pooling_max_memory_protection_keys.is_some()
            && !self.opts.pooling_memory_protection_keys.is_some()
        {
            anyhow::bail!(
                "max memory protection keys requires memory protection keys to be enabled"
            );
        }

        match_feature! {
            ["async" : self.wasm.async_stack_size]
            size => config.async_stack_size(size),
            _ => err,
        }
        match_feature! {
            ["async" : self.wasm.async_stack_zeroing]
            enable => config.async_stack_zeroing(enable),
            _ => err,
        }

        if let Some(max) = self.wasm.max_wasm_stack {
            config.max_wasm_stack(max);

            // If `-Wasync-stack-size` isn't passed then automatically adjust it
            // to the wasm stack size provided here too. That prevents the need
            // to pass both when one can generally be inferred from the other.
            #[cfg(feature = "async")]
            if self.wasm.async_stack_size.is_none() {
                const DEFAULT_HOST_STACK: usize = 512 << 10;
                config.async_stack_size(max + DEFAULT_HOST_STACK);
            }
        }

        if let Some(enable) = self.wasm.relaxed_simd_deterministic {
            config.relaxed_simd_deterministic(enable);
        }
        match_feature! {
            ["cranelift" : self.wasm.wmemcheck]
            enable => config.wmemcheck(enable),
            true => err,
        }

        Ok(config)
    }

    pub fn enable_wasm_features(&self, config: &mut Config) -> Result<()> {
        let all = self.wasm.all_proposals;

        if let Some(enable) = self.wasm.simd.or(all) {
            config.wasm_simd(enable);
        }
        if let Some(enable) = self.wasm.relaxed_simd.or(all) {
            config.wasm_relaxed_simd(enable);
        }
        if let Some(enable) = self.wasm.bulk_memory.or(all) {
            config.wasm_bulk_memory(enable);
        }
        if let Some(enable) = self.wasm.multi_value.or(all) {
            config.wasm_multi_value(enable);
        }
        if let Some(enable) = self.wasm.tail_call.or(all) {
            config.wasm_tail_call(enable);
        }
        if let Some(enable) = self.wasm.multi_memory.or(all) {
            config.wasm_multi_memory(enable);
        }
        if let Some(enable) = self.wasm.memory64.or(all) {
            config.wasm_memory64(enable);
        }
        if let Some(enable) = self.wasm.custom_page_sizes.or(all) {
            config.wasm_custom_page_sizes(enable);
        }
        if let Some(enable) = self.wasm.wide_arithmetic.or(all) {
            config.wasm_wide_arithmetic(enable);
        }
        if let Some(enable) = self.wasm.extended_const.or(all) {
            config.wasm_extended_const(enable);
        }

        macro_rules! handle_conditionally_compiled {
            ($(($feature:tt, $field:tt, $method:tt))*) => ($(
                if let Some(enable) = self.wasm.$field.or(all) {
                    #[cfg(feature = $feature)]
                    config.$method(enable);
                    #[cfg(not(feature = $feature))]
                    if enable && all.is_none() {
                        anyhow::bail!("support for {} was disabled at compile-time", $feature);
                    }
                }
            )*)
        }

        handle_conditionally_compiled! {
            ("component-model", component_model, wasm_component_model)
            ("component-model", component_model_more_flags, wasm_component_model_more_flags)
            ("component-model", component_model_multiple_returns, wasm_component_model_multiple_returns)
            ("component-model-async", component_model_async, wasm_component_model_async)
            ("threads", threads, wasm_threads)
            ("gc", gc, wasm_gc)
            ("gc", reference_types, wasm_reference_types)
            ("gc", function_references, wasm_function_references)
        }
        Ok(())
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let file_contents = fs::read_to_string(path_ref)
            .with_context(|| format!("failed to read config file: {path_ref:?}"))?;
        toml::from_str::<CommonOptions>(&file_contents)
            .with_context(|| format!("failed to parse TOML config file {path_ref:?}"))
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::{OptLevel, RegallocAlgorithm};

    use super::*;

    #[test]
    fn from_toml() {
        // empty toml
        let empty_toml = "";
        let mut common_options: CommonOptions = toml::from_str(empty_toml).unwrap();
        common_options.config(None).unwrap();

        // basic toml
        let basic_toml = r#"
            [optimize]
            [codegen]
            [debug]
            [wasm]
            [wasi]
        "#;
        let mut common_options: CommonOptions = toml::from_str(basic_toml).unwrap();
        common_options.config(None).unwrap();

        // toml with custom deserialization to match CLI flag parsing
        for (opt_value, expected) in [
            ("0", Some(OptLevel::None)),
            ("1", Some(OptLevel::Speed)),
            ("2", Some(OptLevel::Speed)),
            ("\"s\"", Some(OptLevel::SpeedAndSize)),
            ("\"hello\"", None), // should fail
            ("3", None),         // should fail
        ] {
            let toml = format!(
                r#"
                    [optimize]
                    opt-level = {opt_value}
                "#,
            );
            let parsed_opt_level = toml::from_str::<CommonOptions>(&toml)
                .ok()
                .and_then(|common_options| common_options.opts.opt_level);

            assert_eq!(
                parsed_opt_level, expected,
                "Mismatch for input '{opt_value}'. Parsed: {parsed_opt_level:?}, Expected: {expected:?}"
            );
        }

        // Regalloc algorithm
        for (regalloc_value, expected) in [
            ("\"backtracking\"", Some(RegallocAlgorithm::Backtracking)),
            ("\"single-pass\"", Some(RegallocAlgorithm::SinglePass)),
            ("\"hello\"", None), // should fail
            ("3", None),         // should fail
            ("true", None),      // should fail
        ] {
            let toml = format!(
                r#"
                    [optimize]
                    regalloc-algorithm = {regalloc_value}
                "#,
            );
            let parsed_regalloc_algorithm = toml::from_str::<CommonOptions>(&toml)
                .ok()
                .and_then(|common_options| common_options.opts.regalloc_algorithm);
            assert_eq!(
                parsed_regalloc_algorithm, expected,
                "Mismatch for input '{regalloc_value}'. Parsed: {parsed_regalloc_algorithm:?}, Expected: {expected:?}"
            );
        }

        // Strategy
        for (strategy_value, expected) in [
            ("\"cranelift\"", Some(wasmtime::Strategy::Cranelift)),
            ("\"winch\"", Some(wasmtime::Strategy::Winch)),
            ("\"hello\"", None), // should fail
            ("5", None),         // should fail
            ("true", None),      // should fail
        ] {
            let toml = format!(
                r#"
                    [codegen]
                    compiler = {strategy_value}
                "#,
            );
            let parsed_strategy = toml::from_str::<CommonOptions>(&toml)
                .ok()
                .and_then(|common_options| common_options.codegen.compiler);
            assert_eq!(
                parsed_strategy, expected,
                "Mismatch for input '{strategy_value}'. Parsed: {parsed_strategy:?}, Expected: {expected:?}",
            );
        }

        // Collector
        for (collector_value, expected) in [
            (
                "\"drc\"",
                Some(wasmtime::Collector::DeferredReferenceCounting),
            ),
            ("\"null\"", Some(wasmtime::Collector::Null)),
            ("\"hello\"", None), // should fail
            ("5", None),         // should fail
            ("true", None),      // should fail
        ] {
            let toml = format!(
                r#"
                    [codegen]
                    collector = {collector_value}
                "#,
            );
            let parsed_collector = toml::from_str::<CommonOptions>(&toml)
                .ok()
                .and_then(|common_options| common_options.codegen.collector);
            assert_eq!(
                parsed_collector, expected,
                "Mismatch for input '{collector_value}'. Parsed: {parsed_collector:?}, Expected: {expected:?}",
            );
        }
    }
}

impl Default for CommonOptions {
    fn default() -> CommonOptions {
        CommonOptions::new()
    }
}

impl fmt::Display for CommonOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let CommonOptions {
            codegen_raw,
            codegen,
            debug_raw,
            debug,
            opts_raw,
            opts,
            wasm_raw,
            wasm,
            wasi_raw,
            wasi,
            configured,
            target,
            config,
        } = self;
        if let Some(target) = target {
            write!(f, "--target {target} ")?;
        }
        if let Some(config) = config {
            write!(f, "--config {} ", config.display())?;
        }

        let codegen_flags;
        let opts_flags;
        let wasi_flags;
        let wasm_flags;
        let debug_flags;

        if *configured {
            codegen_flags = codegen.to_options();
            debug_flags = debug.to_options();
            wasi_flags = wasi.to_options();
            wasm_flags = wasm.to_options();
            opts_flags = opts.to_options();
        } else {
            codegen_flags = codegen_raw
                .iter()
                .flat_map(|t| t.0.iter())
                .cloned()
                .collect();
            debug_flags = debug_raw.iter().flat_map(|t| t.0.iter()).cloned().collect();
            wasi_flags = wasi_raw.iter().flat_map(|t| t.0.iter()).cloned().collect();
            wasm_flags = wasm_raw.iter().flat_map(|t| t.0.iter()).cloned().collect();
            opts_flags = opts_raw.iter().flat_map(|t| t.0.iter()).cloned().collect();
        }

        for flag in codegen_flags {
            write!(f, "-C{flag} ")?;
        }
        for flag in opts_flags {
            write!(f, "-O{flag} ")?;
        }
        for flag in wasi_flags {
            write!(f, "-S{flag} ")?;
        }
        for flag in wasm_flags {
            write!(f, "-W{flag} ")?;
        }
        for flag in debug_flags {
            write!(f, "-D{flag} ")?;
        }

        Ok(())
    }
}
