//! Contains the common Wasmtime command line interface (CLI) flags.

use anyhow::Result;
use clap::Parser;
use std::time::Duration;
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
    #[derive(PartialEq, Clone)]
    pub struct OptimizeOptions {
        /// Optimization level of generated code (0-2, s; default: 2)
        pub opt_level: Option<wasmtime::OptLevel>,

        /// Byte size of the guard region after dynamic memories are allocated
        pub dynamic_memory_guard_size: Option<u64>,

        /// Force using a "static" style for all wasm memories
        pub static_memory_forced: Option<bool>,

        /// Maximum size in bytes of wasm memory before it becomes dynamically
        /// relocatable instead of up-front-reserved.
        pub static_memory_maximum_size: Option<u64>,

        /// Byte size of the guard region after static memories are allocated
        pub static_memory_guard_size: Option<u64>,

        /// Bytes to reserve at the end of linear memory for growth for dynamic
        /// memories.
        pub dynamic_memory_reserved_for_growth: Option<u64>,

        /// Indicates whether an unmapped region of memory is placed before all
        /// linear memories.
        pub guard_before_linear_memory: Option<bool>,

        /// Enable the pooling allocator, in place of the on-demand allocator.
        pub pooling_allocator: Option<bool>,

        /// How many bytes to keep resident between instantiations for the
        /// pooling allocator in linear memories.
        pub pooling_memory_keep_resident: Option<usize>,

        /// How many bytes to keep resident between instantiations for the
        /// pooling allocator in tables.
        pub pooling_table_keep_resident: Option<usize>,

        /// Enable memory protection keys for the pooling allocator; this can
        /// optimize the size of memory slots.
        pub memory_protection_keys: Option<bool>,

        /// Configure attempting to initialize linear memory via a
        /// copy-on-write mapping (default: yes)
        pub memory_init_cow: Option<bool>,

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

        /// Whether to enable call-indirect caching.
        pub cache_call_indirects: Option<bool>,

        /// The maximum call-indirect cache slot count.
        ///
        /// One slot is allocated per indirect callsite; if the module
        /// has more indirect callsites than this limit, then the
        /// first callsites in linear order in the code section, up to
        /// the limit, will receive a cache slot.
        pub max_call_indirect_cache_slots: Option<usize>,
    }

    enum Optimize {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone)]
    pub struct CodegenOptions {
        /// Either `cranelift` or `winch`.
        ///
        /// Currently only `cranelift` and `winch` are supported, but not all
        /// builds of Wasmtime have both built in.
        pub compiler: Option<wasmtime::Strategy>,
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

        #[prefixed = "cranelift"]
        /// Set a cranelift-specific option. Use `wasmtime settings` to see
        /// all.
        pub cranelift: Vec<(String, Option<String>)>,
    }

    enum Codegen {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone)]
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
    #[derive(PartialEq, Clone)]
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
        pub max_table_elements: Option<u32>,
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
        /// Configure support for the function-references proposal.
        pub function_references: Option<bool>,
        /// Configure support for the GC proposal.
        pub gc: Option<bool>,
    }

    enum Wasm {
        ...
    }
}

wasmtime_option_group! {
    #[derive(PartialEq, Clone)]
    pub struct WasiOptions {
        /// Enable support for WASI CLI APIs, including filesystems, sockets, clocks, and random.
        pub cli: Option<bool>,
        /// Deprecated alias for `cli`
        pub common: Option<bool>,
        /// Enable support for WASI neural network API (experimental)
        pub nn: Option<bool>,
        /// Enable support for WASI threading API (experimental)
        pub threads: Option<bool>,
        /// Enable support for WASI HTTP API (experimental)
        pub http: Option<bool>,
        /// Inherit environment variables and file descriptors following the
        /// systemd listen fd specification (UNIX only)
        pub listenfd: Option<bool>,
        /// Grant access to the given TCP listen socket
        pub tcplisten: Vec<String>,
        /// Implement WASI CLI APIs with preview2 primitives (experimental).
        ///
        /// Indicates that the implementation of WASI preview1 should be backed by
        /// the preview2 implementation for components.
        ///
        /// This will become the default in the future and this option will be
        /// removed. For now this is primarily here for testing.
        pub preview2: Option<bool>,
        /// Pre-load machine learning graphs (i.e., models) for use by wasi-nn.
        ///
        /// Each use of the flag will preload a ML model from the host directory
        /// using the given model encoding. The model will be mapped to the
        /// directory name: e.g., `--wasi-nn-graph openvino:/foo/bar` will preload
        /// an OpenVINO model named `bar`. Note that which model encodings are
        /// available is dependent on the backends implemented in the
        /// `wasmtime_wasi_nn` crate.
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
        /// Allows imports from the `wasi_unstable` core wasm module.
        pub preview0: Option<bool>,
        /// Inherit all environment variables from the parent process.
        ///
        /// This option can be further overwritten with `--env` flags.
        pub inherit_env: Option<bool>,
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

/// Common options for commands that translate WebAssembly modules
#[derive(Parser, Clone)]
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
    opts_raw: Vec<opt::CommaSeparated<Optimize>>,

    /// Codegen-related configuration options, `-C help` to see all.
    #[arg(short = 'C', long = "codegen", value_name = "KEY[=VAL[,..]]")]
    codegen_raw: Vec<opt::CommaSeparated<Codegen>>,

    /// Debug-related configuration options, `-D help` to see all.
    #[arg(short = 'D', long = "debug", value_name = "KEY[=VAL[,..]]")]
    debug_raw: Vec<opt::CommaSeparated<Debug>>,

    /// Options for configuring semantic execution of WebAssembly, `-W help` to see
    /// all.
    #[arg(short = 'W', long = "wasm", value_name = "KEY[=VAL[,..]]")]
    wasm_raw: Vec<opt::CommaSeparated<Wasm>>,

    /// Options for configuring WASI and its proposals, `-S help` to see all.
    #[arg(short = 'S', long = "wasi", value_name = "KEY[=VAL[,..]]")]
    wasi_raw: Vec<opt::CommaSeparated<Wasi>>,

    // These fields are filled in by the `configure` method below via the
    // options parsed from the CLI above. This is what the CLI should use.
    #[arg(skip)]
    configured: bool,
    #[arg(skip)]
    pub opts: OptimizeOptions,
    #[arg(skip)]
    pub codegen: CodegenOptions,
    #[arg(skip)]
    pub debug: DebugOptions,
    #[arg(skip)]
    pub wasm: WasmOptions,
    #[arg(skip)]
    pub wasi: WasiOptions,
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
    fn configure(&mut self) {
        if self.configured {
            return;
        }
        self.configured = true;
        self.opts.configure_with(&self.opts_raw);
        self.codegen.configure_with(&self.codegen_raw);
        self.debug.configure_with(&self.debug_raw);
        self.wasm.configure_with(&self.wasm_raw);
        self.wasi.configure_with(&self.wasi_raw);
    }

    pub fn init_logging(&mut self) -> Result<()> {
        self.configure();
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
            let b = FmtSubscriber::builder()
                .with_writer(std::io::stderr)
                .with_env_filter(EnvFilter::from_env("WASMTIME_LOG"))
                .with_ansi(std::io::stderr().is_terminal());
            b.init();
        }
        #[cfg(not(feature = "logging"))]
        if self.debug.log_to_files == Some(true) || self.debug.logging == Some(true) {
            anyhow::bail!("support for logging disabled at compile time");
        }
        Ok(())
    }

    pub fn config(&mut self, target: Option<&str>) -> Result<Config> {
        self.configure();
        let mut config = Config::new();

        match_feature! {
            ["cranelift" : self.codegen.compiler]
            strategy => config.strategy(strategy),
            _ => err,
        }
        match_feature! {
            ["cranelift" : target]
            target => config.target(target)?,
            _ => err,
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

        if let Some(max) = self.opts.static_memory_maximum_size {
            config.static_memory_maximum_size(max);
        }

        if let Some(enable) = self.opts.static_memory_forced {
            config.static_memory_forced(enable);
        }

        if let Some(size) = self.opts.static_memory_guard_size {
            config.static_memory_guard_size(size);
        }

        if let Some(size) = self.opts.dynamic_memory_guard_size {
            config.dynamic_memory_guard_size(size);
        }
        if let Some(size) = self.opts.dynamic_memory_reserved_for_growth {
            config.dynamic_memory_reserved_for_growth(size);
        }
        if let Some(enable) = self.opts.guard_before_linear_memory {
            config.guard_before_linear_memory(enable);
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
        if let Some(enable) = self.opts.cache_call_indirects {
            config.cache_call_indirects(enable);
        }
        if let Some(max) = self.opts.max_call_indirect_cache_slots {
            config.max_call_indirect_cache_slots(max);
        }

        match_feature! {
            ["pooling-allocator" : self.opts.pooling_allocator]
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
                    if let Some(limit) = self.opts.pooling_total_stacks {
                        cfg.total_stacks(limit);
                    }
                    if let Some(enable) = self.opts.memory_protection_keys {
                        if enable {
                            cfg.memory_protection_keys(wasmtime::MpkEnabled::Enable);
                        }
                    }
                    config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(cfg));
                }
            },
            true => err,
        }

        if self.opts.memory_protection_keys.unwrap_or(false)
            && !self.opts.pooling_allocator.unwrap_or(false)
        {
            anyhow::bail!("memory protection keys require the pooling allocator");
        }

        if let Some(max) = self.wasm.max_wasm_stack {
            config.max_wasm_stack(max);
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
            ("threads", threads, wasm_threads)
            ("gc", gc, wasm_gc)
            ("gc", reference_types, wasm_reference_types)
            ("gc", function_references, wasm_function_references)
        }
        Ok(())
    }
}

impl PartialEq for CommonOptions {
    fn eq(&self, other: &CommonOptions) -> bool {
        let mut me = self.clone();
        me.configure();
        let mut other = other.clone();
        other.configure();
        let CommonOptions {
            opts_raw: _,
            codegen_raw: _,
            debug_raw: _,
            wasm_raw: _,
            wasi_raw: _,
            configured: _,

            opts,
            codegen,
            debug,
            wasm,
            wasi,
        } = me;
        opts == other.opts
            && codegen == other.codegen
            && debug == other.debug
            && wasm == other.wasm
            && wasi == other.wasi
    }
}
