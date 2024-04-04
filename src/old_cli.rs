#![allow(missing_docs)]

use anyhow::{bail, Result};
use clap::builder::{OsStringValueParser, TypedValueParser};
use clap::Parser;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

/// Wasmtime WebAssembly Runtime
#[derive(Parser)]
#[command(
    version,
    after_help = "If a subcommand is not provided, the `run` subcommand will be used.\n\
                  \n\
                  Usage examples:\n\
                  \n\
                  Running a WebAssembly module with a start function:\n\
                  \n  \
                  wasmtime example.wasm
                  \n\
                  Passing command line arguments to a WebAssembly module:\n\
                  \n  \
                  wasmtime example.wasm arg1 arg2 arg3\n\
                  \n\
                  Invoking a specific function (e.g. `add`) in a WebAssembly module:\n\
                  \n  \
                  wasmtime example.wasm --invoke add 1 2\n"
)]
pub enum Wasmtime {
    /// Compiles a WebAssembly module.
    Compile(CompileCommand),
    /// Runs a WebAssembly module
    Run(RunCommand),
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Wasmtime::command().debug_assert()
}

#[derive(Parser)]
pub enum Subcommand {
    /// Compiles a WebAssembly module.
    Compile(CompileCommand),
    /// Runs a WebAssembly module
    Run(RunCommand),
}

#[derive(Parser)]
#[command(trailing_var_arg = true)]
pub struct RunCommand {
    #[command(flatten)]
    common: CommonOptions,

    /// Allow unknown exports when running commands.
    #[arg(long = "allow-unknown-exports")]
    allow_unknown_exports: bool,

    /// Allow the main module to import unknown functions, using an
    /// implementation that immediately traps, when running commands.
    #[arg(long = "trap-unknown-imports")]
    trap_unknown_imports: bool,

    /// Allow the main module to import unknown functions, using an
    /// implementation that returns default values, when running commands.
    #[arg(long = "default-values-unknown-imports")]
    default_values_unknown_imports: bool,

    /// Allow executing precompiled WebAssembly modules as `*.cwasm` files.
    ///
    /// Note that this option is not safe to pass if the module being passed in
    /// is arbitrary user input. Only `wasmtime`-precompiled modules generated
    /// via the `wasmtime compile` command or equivalent should be passed as an
    /// argument with this option specified.
    #[arg(long = "allow-precompiled")]
    allow_precompiled: bool,

    /// Inherit environment variables and file descriptors following the
    /// systemd listen fd specification (UNIX only)
    #[arg(long = "listenfd")]
    listenfd: bool,

    /// Grant access to the given TCP listen socket
    #[arg(
        long = "tcplisten",
        number_of_values = 1,
        value_name = "SOCKET ADDRESS"
    )]
    tcplisten: Vec<String>,

    /// Grant access to the given host directory
    #[arg(long = "dir", number_of_values = 1, value_name = "DIRECTORY")]
    dirs: Vec<String>,

    /// Pass an environment variable to the program.
    ///
    /// The `--env FOO=BAR` form will set the environment variable named `FOO`
    /// to the value `BAR` for the guest program using WASI. The `--env FOO`
    /// form will set the environment variable named `FOO` to the same value it
    /// has in the calling process for the guest, or in other words it will
    /// cause the environment variable `FOO` to be inherited.
    #[arg(long = "env", number_of_values = 1, value_name = "NAME[=VAL]", value_parser = parse_env_var)]
    vars: Vec<(String, Option<String>)>,

    /// The name of the function to run
    #[arg(long, value_name = "FUNCTION")]
    invoke: Option<String>,

    /// Grant access to a guest directory mapped as a host directory
    #[arg(long = "mapdir", number_of_values = 1, value_name = "GUEST_DIR::HOST_DIR", value_parser = parse_map_dirs)]
    map_dirs: Vec<(String, String)>,

    /// Pre-load machine learning graphs (i.e., models) for use by wasi-nn.
    ///
    /// Each use of the flag will preload a ML model from the host directory
    /// using the given model encoding. The model will be mapped to the
    /// directory name: e.g., `--wasi-nn-graph openvino:/foo/bar` will preload
    /// an OpenVINO model named `bar`. Note that which model encodings are
    /// available is dependent on the backends implemented in the
    /// `wasmtime_wasi_nn` crate.
    #[arg(long = "wasi-nn-graph", value_name = "FORMAT::HOST_DIR", value_parser = parse_graphs)]
    graphs: Vec<(String, String)>,

    /// The path of the WebAssembly module to run
    #[arg(
		required = true,
        value_name = "MODULE",
        value_parser = OsStringValueParser::new().try_map(parse_module),
    )]
    module: PathBuf,

    /// Load the given WebAssembly module before the main module
    #[arg(
        long = "preload",
        number_of_values = 1,
        value_name = "NAME=MODULE_PATH",
        value_parser = parse_preloads,
    )]
    preloads: Vec<(String, PathBuf)>,

    /// Maximum execution time of wasm code before timing out (1, 2s, 100ms, etc)
    #[arg(
        long = "wasm-timeout",
        value_name = "TIME",
        value_parser = parse_dur,
    )]
    wasm_timeout: Option<Duration>,

    /// Profiling strategy (valid options are: perfmap, jitdump, vtune, guest)
    ///
    /// The perfmap, jitdump, and vtune profiling strategies integrate Wasmtime
    /// with external profilers such as `perf`. The guest profiling strategy
    /// enables in-process sampling and will write the captured profile to
    /// `wasmtime-guest-profile.json` by default which can be viewed at
    /// <https://profiler.firefox.com/>.
    ///
    /// The `guest` option can be additionally configured as:
    ///
    ///     --profile=guest[,path[,interval]]
    ///
    /// where `path` is where to write the profile and `interval` is the
    /// duration between samples. When used with `--wasm-timeout` the timeout
    /// will be rounded up to the nearest multiple of this interval.
    #[arg(
        long,
        value_name = "STRATEGY",
        value_parser = parse_profile,
    )]
    profile: Option<Profile>,

    /// Enable coredump generation after a WebAssembly trap.
    #[arg(long = "coredump-on-trap", value_name = "PATH")]
    coredump_on_trap: Option<String>,

    // NOTE: this must come last for trailing varargs
    /// The arguments to pass to the module
    #[arg(value_name = "ARGS")]
    module_args: Vec<String>,

    /// Maximum size, in bytes, that a linear memory is allowed to reach.
    ///
    /// Growth beyond this limit will cause `memory.grow` instructions in
    /// WebAssembly modules to return -1 and fail.
    #[arg(long, value_name = "BYTES")]
    max_memory_size: Option<usize>,

    /// Maximum size, in table elements, that a table is allowed to reach.
    #[arg(long)]
    max_table_elements: Option<u32>,

    /// Maximum number of WebAssembly instances allowed to be created.
    #[arg(long)]
    max_instances: Option<usize>,

    /// Maximum number of WebAssembly tables allowed to be created.
    #[arg(long)]
    max_tables: Option<usize>,

    /// Maximum number of WebAssembly linear memories allowed to be created.
    #[arg(long)]
    max_memories: Option<usize>,

    /// Force a trap to be raised on `memory.grow` and `table.grow` failure
    /// instead of returning -1 from these instructions.
    ///
    /// This is not necessarily a spec-compliant option to enable but can be
    /// useful for tracking down a backtrace of what is requesting so much
    /// memory, for example.
    #[arg(long)]
    trap_on_grow_failure: bool,

    /// Indicates that the implementation of WASI preview1 should be backed by
    /// the preview2 implementation for components.
    ///
    /// This will become the default in the future and this option will be
    /// removed. For now this is primarily here for testing.
    #[arg(long)]
    preview2: bool,

    /// Enables memory error checking.
    ///
    /// See wmemcheck.md for documentation on how to use.
    #[arg(long)]
    wmemcheck: bool,

    /// Flag for WASI preview2 to inherit the host's network within the guest so
    /// it has full access to all addresses/ports/etc.
    #[arg(long)]
    inherit_network: bool,
}
fn parse_module(s: OsString) -> anyhow::Result<PathBuf> {
    // Do not accept wasmtime subcommand names as the module name
    match s.to_str() {
        Some("help") | Some("run") | Some("compile") | Some("serve") | Some("explore")
        | Some("settings") | Some("wast") | Some("config") => {
            bail!("module name cannot be the same as a subcommand")
        }
        _ => Ok(s.into()),
    }
}

#[derive(Clone)]
pub enum Profile {
    Native(wasmtime::ProfilingStrategy),
    Guest { path: String, interval: Duration },
}

fn parse_env_var(s: &str) -> Result<(String, Option<String>)> {
    let mut parts = s.splitn(2, '=');
    Ok((
        parts.next().unwrap().to_string(),
        parts.next().map(|s| s.to_string()),
    ))
}

fn parse_map_dirs(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() != 2 {
        bail!("must contain exactly one double colon ('::')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

fn parse_graphs(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() != 2 {
        bail!("must contain exactly one double colon ('::')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

fn parse_dur(s: &str) -> Result<Duration> {
    // assume an integer without a unit specified is a number of seconds ...
    if let Ok(val) = s.parse() {
        return Ok(Duration::from_secs(val));
    }
    // ... otherwise try to parse it with units such as `3s` or `300ms`
    let dur = humantime::parse_duration(s)?;
    Ok(dur)
}

fn parse_preloads(s: &str) -> Result<(String, PathBuf)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("must contain exactly one equals character ('=')");
    }
    Ok((parts[0].into(), parts[1].into()))
}

fn parse_profile(s: &str) -> Result<Profile> {
    let parts = s.split(',').collect::<Vec<_>>();
    match &parts[..] {
        ["perfmap"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::PerfMap)),
        ["jitdump"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::JitDump)),
        ["vtune"] => Ok(Profile::Native(wasmtime::ProfilingStrategy::VTune)),
        ["guest"] => Ok(Profile::Guest {
            path: "wasmtime-guest-profile.json".to_string(),
            interval: Duration::from_millis(10),
        }),
        ["guest", path] => Ok(Profile::Guest {
            path: path.to_string(),
            interval: Duration::from_millis(10),
        }),
        ["guest", path, dur] => Ok(Profile::Guest {
            path: path.to_string(),
            interval: parse_dur(dur)?,
        }),
        _ => bail!("unknown profiling strategy: {s}"),
    }
}

/// Compiles a WebAssembly module.
#[derive(Parser)]
pub struct CompileCommand {
    #[command(flatten)]
    pub common: CommonOptions,

    /// The target triple; default is the host triple
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// The path of the output compiled module; defaults to `<MODULE>.cwasm`
    #[arg(short = 'o', long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// The directory path to write clif files into, one clif file per wasm function.
    #[arg(long = "emit-clif", value_name = "PATH")]
    pub emit_clif: Option<PathBuf>,

    /// The path of the WebAssembly to compile
    #[arg(index = 1, value_name = "MODULE")]
    pub module: PathBuf,
}

/// Common options for commands that translate WebAssembly modules
#[derive(Parser)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct CommonOptions {
    /// Use specified configuration file
    #[arg(long, value_name = "CONFIG_PATH")]
    pub config: Option<PathBuf>,

    /// Disable logging
    #[arg(long, conflicts_with = "log_to_files")]
    pub disable_logging: bool,

    /// Log to per-thread log files instead of stderr
    #[arg(long)]
    pub log_to_files: bool,

    /// Generate debug information
    #[arg(short = 'g')]
    pub debug_info: bool,

    /// Disable cache system
    #[arg(long)]
    pub disable_cache: bool,

    /// Disable parallel compilation
    #[arg(long)]
    pub disable_parallel_compilation: bool,

    /// Enable or disable WebAssembly features
    #[arg(long, value_name = "FEATURE,FEATURE,...", value_parser = parse_wasm_features)]
    pub wasm_features: Option<WasmFeatures>,

    /// Enable or disable WASI modules
    #[arg(long, value_name = "MODULE,MODULE,...", value_parser = parse_wasi_modules)]
    pub wasi_modules: Option<WasiModules>,

    /// Generate jitdump file (supported on --features=profiling build)
    /// Run optimization passes on translated functions, on by default
    #[arg(short = 'O', long)]
    pub optimize: bool,

    /// Optimization level for generated functions
    /// Supported levels: 0 (none), 1, 2 (most), or s (size); default is "most"
    #[arg(
        long,
        value_name = "LEVEL",
        value_parser = parse_opt_level,
        verbatim_doc_comment,
    )]
    pub opt_level: Option<wasmtime::OptLevel>,

    /// Set a Cranelift setting to a given value.
    /// Use `wasmtime settings` to list Cranelift settings for a target.
    #[arg(
        long = "cranelift-set",
        value_name = "NAME=VALUE",
        number_of_values = 1,
        verbatim_doc_comment,
        value_parser = parse_cranelift_flag,
    )]
    pub cranelift_set: Vec<(String, String)>,

    /// Enable a Cranelift boolean setting or preset.
    /// Use `wasmtime settings` to list Cranelift settings for a target.
    #[arg(
        long,
        value_name = "SETTING",
        number_of_values = 1,
        verbatim_doc_comment
    )]
    pub cranelift_enable: Vec<String>,

    /// Maximum size in bytes of wasm memory before it becomes dynamically
    /// relocatable instead of up-front-reserved.
    #[arg(long, value_name = "MAXIMUM")]
    pub static_memory_maximum_size: Option<u64>,

    /// Force using a "static" style for all wasm memories
    #[arg(long)]
    pub static_memory_forced: bool,

    /// Byte size of the guard region after static memories are allocated
    #[arg(long, value_name = "SIZE")]
    pub static_memory_guard_size: Option<u64>,

    /// Byte size of the guard region after dynamic memories are allocated
    #[arg(long, value_name = "SIZE")]
    pub dynamic_memory_guard_size: Option<u64>,

    /// Bytes to reserve at the end of linear memory for growth for dynamic
    /// memories.
    #[arg(long, value_name = "SIZE")]
    pub dynamic_memory_reserved_for_growth: Option<u64>,

    /// Enable Cranelift's internal debug verifier (expensive)
    #[arg(long)]
    pub enable_cranelift_debug_verifier: bool,

    /// Enable Cranelift's internal NaN canonicalization
    #[arg(long)]
    pub enable_cranelift_nan_canonicalization: bool,

    /// Enable execution fuel with N units fuel, where execution will trap after
    /// running out of fuel.
    ///
    /// Most WebAssembly instructions consume 1 unit of fuel. Some instructions,
    /// such as `nop`, `drop`, `block`, and `loop`, consume 0 units, as any
    /// execution cost associated with them involves other instructions which do
    /// consume fuel.
    #[arg(long, value_name = "N")]
    pub fuel: Option<u64>,

    /// Executing wasm code will yield when a global epoch counter
    /// changes, allowing for async operation without blocking the
    /// executor.
    #[arg(long)]
    pub epoch_interruption: bool,

    /// Disable the on-by-default address map from native code to wasm code
    #[arg(long)]
    pub disable_address_map: bool,

    /// Disable the default of attempting to initialize linear memory via a
    /// copy-on-write mapping
    #[arg(long)]
    pub disable_memory_init_cow: bool,

    /// Enable the pooling allocator, in place of the on-demand
    /// allocator.
    #[cfg(feature = "pooling-allocator")]
    #[arg(long)]
    pub pooling_allocator: bool,

    /// Maximum stack size, in bytes, that wasm is allowed to consume before a
    /// stack overflow is reported.
    #[arg(long)]
    pub max_wasm_stack: Option<usize>,

    /// Whether or not to force deterministic and host-independent behavior of
    /// the relaxed-simd instructions.
    ///
    /// By default these instructions may have architecture-specific behavior as
    /// allowed by the specification, but this can be used to force the behavior
    /// of these instructions to match the deterministic behavior classified in
    /// the specification. Note that enabling this option may come at a
    /// performance cost.
    #[arg(long)]
    pub relaxed_simd_deterministic: bool,

    /// Explicitly specify the name of the compiler to use for WebAssembly.
    ///
    /// Currently only `cranelift` and `winch` are supported, but not all builds
    /// of Wasmtime have both built in.
    #[arg(long)]
    pub compiler: Option<String>,
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
    pub component_model: Option<bool>,
    pub function_references: Option<bool>,
}

const SUPPORTED_WASM_FEATURES: &[(&str, &str)] = &[
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
    ("component-model", "enables support for the component model"),
    (
        "function-references",
        "enables support for typed function references",
    ),
];

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

/// Select which WASI modules are available at runtime for use by Wasm programs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WasiModules {
    /// Enable the wasi-common implementation; eventually this should be split into its separate
    /// parts once the implementation allows for it (e.g. wasi-fs, wasi-clocks, etc.).
    pub wasi_common: Option<bool>,

    /// Enable the experimental wasi-nn implementation.
    pub wasi_nn: Option<bool>,

    /// Enable the experimental wasi-threads implementation.
    pub wasi_threads: Option<bool>,

    /// Enable the experimental wasi-http implementation
    pub wasi_http: Option<bool>,
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
                "wasi-common" => Ok(wasi_modules.wasi_common = Some(enable)),
                "experimental-wasi-nn" => Ok(wasi_modules.wasi_nn = Some(enable)),
                "experimental-wasi-threads" => Ok(wasi_modules.wasi_threads = Some(enable)),
                "experimental-wasi-http" => Ok(wasi_modules.wasi_http = Some(enable)),
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

impl Default for WasiModules {
    fn default() -> Self {
        Self {
            wasi_common: None,
            wasi_nn: None,
            wasi_threads: None,
            wasi_http: None,
        }
    }
}

impl WasiModules {
    /// Enable no modules.
    fn none() -> Self {
        Self {
            wasi_common: Some(false),
            wasi_nn: Some(false),
            wasi_threads: Some(false),
            wasi_http: Some(false),
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

impl CompileCommand {
    pub fn convert(self) -> crate::commands::CompileCommand {
        let CompileCommand {
            common,
            target,
            output,
            emit_clif,
            module,
        } = self;

        crate::commands::CompileCommand {
            common: common.convert(),
            target,
            output,
            emit_clif,
            module,
        }
    }
}

impl RunCommand {
    pub fn convert(self) -> crate::commands::RunCommand {
        let RunCommand {
            common,
            allow_unknown_exports,
            trap_unknown_imports,
            default_values_unknown_imports,
            allow_precompiled,
            listenfd,
            tcplisten,
            dirs: old_dirs,
            vars,
            invoke,
            map_dirs,
            graphs,
            preloads,
            wasm_timeout,
            profile,
            coredump_on_trap,
            max_memory_size,
            max_table_elements,
            max_instances,
            max_tables,
            max_memories,
            trap_on_grow_failure,
            wmemcheck,
            module,
            module_args,
            preview2,
            inherit_network,
        } = self;

        let mut common = common.convert();

        let mut dirs = Vec::new();

        for host in old_dirs {
            let mut parts = host.splitn(2, "::");
            let host = parts.next().unwrap();
            let guest = parts.next().unwrap_or(host);
            dirs.push((host.to_string(), guest.to_string()));
        }

        if preview2 {
            common.wasi.preview2 = Some(true);
        }
        if wmemcheck {
            common.wasm.wmemcheck = Some(true);
        }
        if trap_on_grow_failure {
            common.wasm.trap_on_grow_failure = Some(true);
        }
        if let Some(max) = max_memories {
            common.wasm.max_memories = Some(max);
        }
        if let Some(max) = max_tables {
            common.wasm.max_tables = Some(max);
        }
        if let Some(max) = max_instances {
            common.wasm.max_instances = Some(max);
        }
        if let Some(max) = max_memory_size {
            common.wasm.max_memory_size = Some(max);
        }
        if let Some(max) = max_table_elements {
            common.wasm.max_table_elements = Some(max);
        }
        if let Some(path) = coredump_on_trap {
            common.debug.coredump = Some(path);
        }
        if let Some(timeout) = wasm_timeout {
            common.wasm.timeout = Some(timeout);
        }
        if trap_unknown_imports {
            common.wasm.unknown_imports_trap = Some(true);
        }
        if default_values_unknown_imports {
            common.wasm.unknown_imports_default = Some(true);
        }
        common.wasi.tcplisten = tcplisten;
        if listenfd {
            common.wasi.listenfd = Some(true);
        }
        if allow_unknown_exports {
            common.wasm.unknown_exports_allow = Some(true);
        }
        if inherit_network {
            common.wasi.inherit_network = Some(true);
        }

        for (format, dir) in graphs {
            common
                .wasi
                .nn_graph
                .push(wasmtime_cli_flags::WasiNnGraph { format, dir });
        }

        for (guest, host) in map_dirs {
            dirs.push((host, guest));
        }

        let run = crate::common::RunCommon {
            common,
            allow_precompiled,
            profile: profile.map(|p| p.convert()),
            dirs,
            vars,
        };

        let mut module_and_args = vec![module.into()];
        module_and_args.extend(module_args.into_iter().map(|s| s.into()));
        crate::commands::RunCommand {
            run,
            invoke,
            preloads,
            module_and_args,
        }
    }
}

impl CommonOptions {
    pub fn convert(self) -> wasmtime_cli_flags::CommonOptions {
        let CommonOptions {
            config,
            disable_logging,
            log_to_files,
            debug_info,
            disable_cache,
            disable_parallel_compilation,
            wasm_features,
            wasi_modules,
            optimize,
            opt_level,
            cranelift_set,
            cranelift_enable,
            static_memory_maximum_size,
            static_memory_forced,
            static_memory_guard_size,
            dynamic_memory_guard_size,
            dynamic_memory_reserved_for_growth,
            enable_cranelift_debug_verifier,
            enable_cranelift_nan_canonicalization,
            fuel,
            epoch_interruption,
            disable_address_map,
            disable_memory_init_cow,
            pooling_allocator,
            max_wasm_stack,
            relaxed_simd_deterministic,
            compiler,
        } = self;

        let mut ret = wasmtime_cli_flags::CommonOptions::parse_from::<_, String>([]);
        match compiler.as_deref() {
            Some("cranelift") => ret.codegen.compiler = Some(wasmtime::Strategy::Cranelift),
            Some("winch") => ret.codegen.compiler = Some(wasmtime::Strategy::Winch),

            // Plumbing an error up from this point is a bit onerous. Let's
            // just hope that no one was using this from the old CLI and passing
            // invalid values.
            Some(_) => {}

            None => {}
        }
        if relaxed_simd_deterministic {
            ret.wasm.relaxed_simd_deterministic = Some(true);
        }
        ret.wasm.max_wasm_stack = max_wasm_stack;
        if pooling_allocator {
            ret.opts.pooling_allocator = Some(true);
        }
        if disable_memory_init_cow {
            ret.opts.memory_init_cow = Some(false);
        }
        if disable_address_map {
            ret.debug.address_map = Some(false);
        }
        if epoch_interruption {
            ret.wasm.epoch_interruption = Some(true);
        }
        if enable_cranelift_debug_verifier {
            ret.codegen.cranelift_debug_verifier = Some(true);
        }
        if enable_cranelift_nan_canonicalization {
            ret.wasm.nan_canonicalization = Some(true);
        }
        if let Some(fuel) = fuel {
            ret.wasm.fuel = Some(fuel);
        }
        if let Some(amt) = dynamic_memory_reserved_for_growth {
            ret.opts.dynamic_memory_reserved_for_growth = Some(amt);
        }
        if let Some(amt) = dynamic_memory_guard_size {
            ret.opts.dynamic_memory_guard_size = Some(amt);
        }
        if let Some(amt) = static_memory_guard_size {
            ret.opts.static_memory_guard_size = Some(amt);
        }
        if static_memory_forced {
            ret.opts.static_memory_forced = Some(true);
        }
        if let Some(amt) = static_memory_maximum_size {
            ret.opts.static_memory_maximum_size = Some(amt);
        }
        if let Some(level) = opt_level {
            ret.opts.opt_level = Some(level);
        }
        if disable_cache {
            ret.codegen.cache = Some(false);
        }
        if debug_info {
            ret.debug.debug_info = Some(true);
        }
        if optimize {
            ret.opts.opt_level = Some(wasmtime::OptLevel::Speed);
        }
        if disable_parallel_compilation {
            ret.codegen.parallel_compilation = Some(false);
        }
        if log_to_files {
            ret.debug.log_to_files = Some(true);
        }
        if disable_logging {
            ret.debug.logging = Some(false);
        }
        if let Some(path) = config {
            ret.codegen.cache_config = Some(path.display().to_string());
        }
        for (key, val) in cranelift_set {
            ret.codegen.cranelift.push((key, Some(val)));
        }
        for key in cranelift_enable {
            ret.codegen.cranelift.push((key, None));
        }
        if let Some(features) = wasm_features {
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
                component_model,
                function_references,
            } = features;
            ret.wasm.reference_types = reference_types;
            ret.wasm.multi_value = multi_value;
            ret.wasm.bulk_memory = bulk_memory;
            ret.wasm.simd = simd;
            ret.wasm.relaxed_simd = relaxed_simd;
            ret.wasm.tail_call = tail_call;
            ret.wasm.threads = threads;
            ret.wasm.multi_memory = multi_memory;
            ret.wasm.memory64 = memory64;
            ret.wasm.component_model = component_model;
            ret.wasm.function_references = function_references;
        }
        if let Some(modules) = wasi_modules {
            let WasiModules {
                wasi_common,
                wasi_nn,
                wasi_threads,
                wasi_http,
            } = modules;
            ret.wasi.http = wasi_http;
            ret.wasi.nn = wasi_nn;
            ret.wasi.threads = wasi_threads;
            ret.wasi.cli = wasi_common;
        }
        ret
    }
}

impl Profile {
    pub fn convert(self) -> crate::common::Profile {
        match self {
            Profile::Native(s) => crate::common::Profile::Native(s),
            Profile::Guest { path, interval } => crate::common::Profile::Guest { path, interval },
        }
    }
}
