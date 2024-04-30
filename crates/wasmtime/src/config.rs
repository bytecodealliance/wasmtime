use anyhow::{bail, ensure, Result};
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
#[cfg(any(feature = "cache", feature = "cranelift", feature = "winch"))]
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use target_lexicon::Architecture;
use wasmparser::WasmFeatures;
#[cfg(feature = "cache")]
use wasmtime_cache::CacheConfig;
use wasmtime_environ::Tunables;

#[cfg(feature = "runtime")]
use crate::memory::MemoryCreator;
#[cfg(feature = "runtime")]
use crate::profiling_agent::{self, ProfilingAgent};
#[cfg(feature = "runtime")]
use crate::trampoline::MemoryCreatorProxy;
#[cfg(feature = "runtime")]
use wasmtime_runtime::{
    GcRuntime, InstanceAllocator, OnDemandInstanceAllocator, RuntimeMemoryCreator,
};

#[cfg(feature = "async")]
use crate::stack::{StackCreator, StackCreatorProxy};
#[cfg(feature = "async")]
use wasmtime_fiber::RuntimeFiberStackCreator;

#[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
pub use wasmtime_environ::CacheStore;
#[cfg(feature = "pooling-allocator")]
use wasmtime_runtime::mpk;
#[cfg(feature = "pooling-allocator")]
pub use wasmtime_runtime::MpkEnabled;

/// Represents the module instance allocation strategy to use.
#[derive(Clone)]
pub enum InstanceAllocationStrategy {
    /// The on-demand instance allocation strategy.
    ///
    /// Resources related to a module instance are allocated at instantiation time and
    /// immediately deallocated when the `Store` referencing the instance is dropped.
    ///
    /// This is the default allocation strategy for Wasmtime.
    OnDemand,
    /// The pooling instance allocation strategy.
    ///
    /// A pool of resources is created in advance and module instantiation reuses resources
    /// from the pool. Resources are returned to the pool when the `Store` referencing the instance
    /// is dropped.
    #[cfg(feature = "pooling-allocator")]
    Pooling(PoolingAllocationConfig),
}

impl InstanceAllocationStrategy {
    /// The default pooling instance allocation strategy.
    #[cfg(feature = "pooling-allocator")]
    pub fn pooling() -> Self {
        Self::Pooling(Default::default())
    }
}

impl Default for InstanceAllocationStrategy {
    fn default() -> Self {
        Self::OnDemand
    }
}

#[derive(Clone)]
/// Configure the strategy used for versioning in serializing and deserializing [`crate::Module`].
pub enum ModuleVersionStrategy {
    /// Use the wasmtime crate's Cargo package version.
    WasmtimeVersion,
    /// Use a custom version string. Must be at most 255 bytes.
    Custom(String),
    /// Emit no version string in serialization, and accept all version strings in deserialization.
    None,
}

impl Default for ModuleVersionStrategy {
    fn default() -> Self {
        ModuleVersionStrategy::WasmtimeVersion
    }
}

impl std::hash::Hash for ModuleVersionStrategy {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        match self {
            Self::WasmtimeVersion => env!("CARGO_PKG_VERSION").hash(hasher),
            Self::Custom(s) => s.hash(hasher),
            Self::None => {}
        };
    }
}

/// Global configuration options used to create an [`Engine`](crate::Engine)
/// and customize its behavior.
///
/// This structure exposed a builder-like interface and is primarily consumed by
/// [`Engine::new()`](crate::Engine::new).
///
/// The validation of `Config` is deferred until the engine is being built, thus
/// a problematic config may cause `Engine::new` to fail.
#[derive(Clone)]
pub struct Config {
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    compiler_config: CompilerConfig,
    profiling_strategy: ProfilingStrategy,
    tunables: ConfigTunables,

    #[cfg(feature = "cache")]
    pub(crate) cache_config: CacheConfig,
    #[cfg(feature = "runtime")]
    pub(crate) mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>,
    pub(crate) allocation_strategy: InstanceAllocationStrategy,
    pub(crate) max_wasm_stack: usize,
    pub(crate) features: WasmFeatures,
    pub(crate) wasm_backtrace: bool,
    pub(crate) wasm_backtrace_details_env_used: bool,
    pub(crate) native_unwind_info: Option<bool>,
    #[cfg(feature = "async")]
    pub(crate) async_stack_size: usize,
    #[cfg(feature = "async")]
    pub(crate) stack_creator: Option<Arc<dyn RuntimeFiberStackCreator>>,
    pub(crate) async_support: bool,
    pub(crate) module_version: ModuleVersionStrategy,
    pub(crate) parallel_compilation: bool,
    pub(crate) memory_init_cow: bool,
    pub(crate) memory_guaranteed_dense_image_size: u64,
    pub(crate) force_memory_init_memfd: bool,
    pub(crate) wmemcheck: bool,
    pub(crate) coredump_on_trap: bool,
    pub(crate) macos_use_mach_ports: bool,
}

#[derive(Default, Clone)]
struct ConfigTunables {
    static_memory_bound: Option<u64>,
    static_memory_offset_guard_size: Option<u64>,
    dynamic_memory_offset_guard_size: Option<u64>,
    dynamic_memory_growth_reserve: Option<u64>,
    generate_native_debuginfo: Option<bool>,
    parse_wasm_debuginfo: Option<bool>,
    consume_fuel: Option<bool>,
    epoch_interruption: Option<bool>,
    static_memory_bound_is_maximum: Option<bool>,
    guard_before_linear_memory: Option<bool>,
    generate_address_map: Option<bool>,
    debug_adapter_modules: Option<bool>,
    relaxed_simd_deterministic: Option<bool>,
    tail_callable: Option<bool>,
    cache_call_indirects: Option<bool>,
}

/// User-provided configuration for the compiler.
#[cfg(any(feature = "cranelift", feature = "winch"))]
#[derive(Debug, Clone)]
struct CompilerConfig {
    strategy: Strategy,
    target: Option<target_lexicon::Triple>,
    settings: HashMap<String, String>,
    flags: HashSet<String>,
    #[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
    cache_store: Option<Arc<dyn CacheStore>>,
    clif_dir: Option<std::path::PathBuf>,
    wmemcheck: bool,
}

#[cfg(any(feature = "cranelift", feature = "winch"))]
impl CompilerConfig {
    fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            target: None,
            settings: HashMap::new(),
            flags: HashSet::new(),
            #[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
            cache_store: None,
            clif_dir: None,
            wmemcheck: false,
        }
    }

    /// Ensures that the key is not set or equals to the given value.
    /// If the key is not set, it will be set to the given value.
    ///
    /// # Returns
    ///
    /// Returns true if successfully set or already had the given setting
    /// value, or false if the setting was explicitly set to something
    /// else previously.
    fn ensure_setting_unset_or_given(&mut self, k: &str, v: &str) -> bool {
        if let Some(value) = self.settings.get(k) {
            if value != v {
                return false;
            }
        } else {
            self.settings.insert(k.to_string(), v.to_string());
        }
        true
    }
}

#[cfg(any(feature = "cranelift", feature = "winch"))]
impl Default for CompilerConfig {
    fn default() -> Self {
        Self::new(Strategy::Auto)
    }
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        let mut ret = Self {
            tunables: ConfigTunables::default(),
            #[cfg(any(feature = "cranelift", feature = "winch"))]
            compiler_config: CompilerConfig::default(),
            #[cfg(feature = "cache")]
            cache_config: CacheConfig::new_cache_disabled(),
            profiling_strategy: ProfilingStrategy::None,
            #[cfg(feature = "runtime")]
            mem_creator: None,
            allocation_strategy: InstanceAllocationStrategy::OnDemand,
            // 512k of stack -- note that this is chosen currently to not be too
            // big, not be too small, and be a good default for most platforms.
            // One platform of particular note is Windows where the stack size
            // of the main thread seems to, by default, be smaller than that of
            // Linux and macOS. This 512k value at least lets our current test
            // suite pass on the main thread of Windows (using `--test-threads
            // 1` forces this), or at least it passed when this change was
            // committed.
            max_wasm_stack: 512 * 1024,
            wasm_backtrace: true,
            wasm_backtrace_details_env_used: false,
            native_unwind_info: None,
            features: WasmFeatures::default(),
            #[cfg(feature = "async")]
            async_stack_size: 2 << 20,
            #[cfg(feature = "async")]
            stack_creator: None,
            async_support: false,
            module_version: ModuleVersionStrategy::default(),
            parallel_compilation: !cfg!(miri),
            memory_init_cow: true,
            memory_guaranteed_dense_image_size: 16 << 20,
            force_memory_init_memfd: false,
            wmemcheck: false,
            coredump_on_trap: false,
            macos_use_mach_ports: !cfg!(miri),
        };
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        {
            ret.cranelift_debug_verifier(false);
            ret.cranelift_opt_level(OptLevel::Speed);
        }

        // Conditionally enabled features depending on compile-time crate
        // features. Note that if these features are disabled then `Config` has
        // no way of re-enabling them.
        ret.features
            .set(WasmFeatures::REFERENCE_TYPES, cfg!(feature = "gc"));
        ret.features
            .set(WasmFeatures::THREADS, cfg!(feature = "threads"));
        ret.features.set(
            WasmFeatures::COMPONENT_MODEL,
            cfg!(feature = "component-model"),
        );

        // If GC is disabled at compile time also disable it in features
        // forcibly irrespective of `wasmparser` defaults. Note that these also
        // aren't yet fully implemented in Wasmtime.
        if !cfg!(feature = "gc") {
            ret.features.set(WasmFeatures::FUNCTION_REFERENCES, false);
            ret.features.set(WasmFeatures::GC, false);
        }

        ret.wasm_multi_value(true);
        ret.wasm_bulk_memory(true);
        ret.wasm_simd(true);
        ret.wasm_backtrace_details(WasmBacktraceDetails::Environment);

        // This is on-by-default in `wasmparser` since it's a stage 4+ proposal
        // but it's not implemented in Wasmtime yet so disable it.
        ret.features.set(WasmFeatures::TAIL_CALL, false);

        ret
    }

    /// Sets the target triple for the [`Config`].
    ///
    /// By default, the host target triple is used for the [`Config`].
    ///
    /// This method can be used to change the target triple.
    ///
    /// Cranelift flags will not be inferred for the given target and any
    /// existing target-specific Cranelift flags will be cleared.
    ///
    /// # Errors
    ///
    /// This method will error if the given target triple is not supported.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn target(&mut self, target: &str) -> Result<&mut Self> {
        self.compiler_config.target =
            Some(target_lexicon::Triple::from_str(target).map_err(|e| anyhow::anyhow!(e))?);

        Ok(self)
    }

    /// Enables the incremental compilation cache in Cranelift, using the provided `CacheStore`
    /// backend for storage.
    #[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
    pub fn enable_incremental_compilation(
        &mut self,
        cache_store: Arc<dyn CacheStore>,
    ) -> Result<&mut Self> {
        self.compiler_config.cache_store = Some(cache_store);
        Ok(self)
    }

    /// Whether or not to enable support for asynchronous functions in Wasmtime.
    ///
    /// When enabled, the config can optionally define host functions with `async`.
    /// Instances created and functions called with this `Config` *must* be called
    /// through their asynchronous APIs, however. For example using
    /// [`Func::call`](crate::Func::call) will panic when used with this config.
    ///
    /// # Asynchronous Wasm
    ///
    /// WebAssembly does not currently have a way to specify at the bytecode
    /// level what is and isn't async. Host-defined functions, however, may be
    /// defined as `async`. WebAssembly imports always appear synchronous, which
    /// gives rise to a bit of an impedance mismatch here. To solve this
    /// Wasmtime supports "asynchronous configs" which enables calling these
    /// asynchronous functions in a way that looks synchronous to the executing
    /// WebAssembly code.
    ///
    /// An asynchronous config must always invoke wasm code asynchronously,
    /// meaning we'll always represent its computation as a
    /// [`Future`](std::future::Future). The `poll` method of the futures
    /// returned by Wasmtime will perform the actual work of calling the
    /// WebAssembly. Wasmtime won't manage its own thread pools or similar,
    /// that's left up to the embedder.
    ///
    /// To implement futures in a way that WebAssembly sees asynchronous host
    /// functions as synchronous, all async Wasmtime futures will execute on a
    /// separately allocated native stack from the thread otherwise executing
    /// Wasmtime. This separate native stack can then be switched to and from.
    /// Using this whenever an `async` host function returns a future that
    /// resolves to `Pending` we switch away from the temporary stack back to
    /// the main stack and propagate the `Pending` status.
    ///
    /// In general it's encouraged that the integration with `async` and
    /// wasmtime is designed early on in your embedding of Wasmtime to ensure
    /// that it's planned that WebAssembly executes in the right context of your
    /// application.
    ///
    /// # Execution in `poll`
    ///
    /// The [`Future::poll`](std::future::Future::poll) method is the main
    /// driving force behind Rust's futures. That method's own documentation
    /// states "an implementation of `poll` should strive to return quickly, and
    /// should not block". This, however, can be at odds with executing
    /// WebAssembly code as part of the `poll` method itself. If your
    /// WebAssembly is untrusted then this could allow the `poll` method to take
    /// arbitrarily long in the worst case, likely blocking all other
    /// asynchronous tasks.
    ///
    /// To remedy this situation you have a a few possible ways to solve this:
    ///
    /// * The most efficient solution is to enable
    ///   [`Config::epoch_interruption`] in conjunction with
    ///   [`crate::Store::epoch_deadline_async_yield_and_update`]. Coupled with
    ///   periodic calls to [`crate::Engine::increment_epoch`] this will cause
    ///   executing WebAssembly to periodically yield back according to the
    ///   epoch configuration settings. This enables `Future::poll` to take at
    ///   most a certain amount of time according to epoch configuration
    ///   settings and when increments happen. The benefit of this approach is
    ///   that the instrumentation in compiled code is quite lightweight, but a
    ///   downside can be that the scheduling is somewhat nondeterministic since
    ///   increments are usually timer-based which are not always deterministic.
    ///
    ///   Note that to prevent infinite execution of wasm it's recommended to
    ///   place a timeout on the entire future representing executing wasm code
    ///   and the periodic yields with epochs should ensure that when the
    ///   timeout is reached it's appropriately recognized.
    ///
    /// * Alternatively you can enable the
    ///   [`Config::consume_fuel`](crate::Config::consume_fuel) method as well
    ///   as [`crate::Store::fuel_async_yield_interval`] When doing so this will
    ///   configure Wasmtime futures to yield periodically while they're
    ///   executing WebAssembly code. After consuming the specified amount of
    ///   fuel wasm futures will return `Poll::Pending` from their `poll`
    ///   method, and will get automatically re-polled later. This enables the
    ///   `Future::poll` method to take roughly a fixed amount of time since
    ///   fuel is guaranteed to get consumed while wasm is executing. Unlike
    ///   epoch-based preemption this is deterministic since wasm always
    ///   consumes a fixed amount of fuel per-operation. The downside of this
    ///   approach, however, is that the compiled code instrumentation is
    ///   significantly more expensive than epoch checks.
    ///
    ///   Note that to prevent infinite execution of wasm it's recommended to
    ///   place a timeout on the entire future representing executing wasm code
    ///   and the periodic yields with epochs should ensure that when the
    ///   timeout is reached it's appropriately recognized.
    ///
    /// In all cases special care needs to be taken when integrating
    /// asynchronous wasm into your application. You should carefully plan where
    /// WebAssembly will execute and what compute resources will be allotted to
    /// it. If Wasmtime doesn't support exactly what you'd like just yet, please
    /// feel free to open an issue!
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn async_support(&mut self, enable: bool) -> &mut Self {
        self.async_support = enable;
        self
    }

    /// Configures whether DWARF debug information will be emitted during
    /// compilation.
    ///
    /// Note that the `debug-builtins` compile-time Cargo feature must also be
    /// enabled for native debuggers such as GDB or LLDB to be able to debug
    /// guest WebAssembly programs.
    ///
    /// By default this option is `false`.
    pub fn debug_info(&mut self, enable: bool) -> &mut Self {
        self.tunables.generate_native_debuginfo = Some(enable);
        self
    }

    /// Configures whether [`WasmBacktrace`] will be present in the context of
    /// errors returned from Wasmtime.
    ///
    /// A backtrace may be collected whenever an error is returned from a host
    /// function call through to WebAssembly or when WebAssembly itself hits a
    /// trap condition, such as an out-of-bounds memory access. This flag
    /// indicates, in these conditions, whether the backtrace is collected or
    /// not.
    ///
    /// Currently wasm backtraces are implemented through frame pointer walking.
    /// This means that collecting a backtrace is expected to be a fast and
    /// relatively cheap operation. Additionally backtrace collection is
    /// suitable in concurrent environments since one thread capturing a
    /// backtrace won't block other threads.
    ///
    /// Collected backtraces are attached via [`anyhow::Error::context`] to
    /// errors returned from host functions. The [`WasmBacktrace`] type can be
    /// acquired via [`anyhow::Error::downcast_ref`] to inspect the backtrace.
    /// When this option is disabled then this context is never applied to
    /// errors coming out of wasm.
    ///
    /// This option is `true` by default.
    ///
    /// [`WasmBacktrace`]: crate::WasmBacktrace
    pub fn wasm_backtrace(&mut self, enable: bool) -> &mut Self {
        self.wasm_backtrace = enable;
        self
    }

    /// Configures whether backtraces in `Trap` will parse debug info in the wasm file to
    /// have filename/line number information.
    ///
    /// When enabled this will causes modules to retain debugging information
    /// found in wasm binaries. This debug information will be used when a trap
    /// happens to symbolicate each stack frame and attempt to print a
    /// filename/line number for each wasm frame in the stack trace.
    ///
    /// By default this option is `WasmBacktraceDetails::Environment`, meaning
    /// that wasm will read `WASMTIME_BACKTRACE_DETAILS` to indicate whether details
    /// should be parsed.
    pub fn wasm_backtrace_details(&mut self, enable: WasmBacktraceDetails) -> &mut Self {
        self.wasm_backtrace_details_env_used = false;
        self.tunables.parse_wasm_debuginfo = match enable {
            WasmBacktraceDetails::Enable => Some(true),
            WasmBacktraceDetails::Disable => Some(false),
            WasmBacktraceDetails::Environment => {
                self.wasm_backtrace_details_env_used = true;
                std::env::var("WASMTIME_BACKTRACE_DETAILS")
                    .map(|s| Some(s == "1"))
                    .unwrap_or(Some(false))
            }
        };
        self
    }

    /// Configures whether to generate native unwind information
    /// (e.g. `.eh_frame` on Linux).
    ///
    /// This configuration option only exists to help third-party stack
    /// capturing mechanisms, such as the system's unwinder or the `backtrace`
    /// crate, determine how to unwind through Wasm frames. It does not affect
    /// whether Wasmtime can capture Wasm backtraces or not. The presence of
    /// [`WasmBacktrace`] is controlled by the [`Config::wasm_backtrace`]
    /// option.
    ///
    /// Native unwind information is included:
    /// - When targeting Windows, since the Windows ABI requires it.
    /// - By default.
    ///
    /// [`WasmBacktrace`]: crate::WasmBacktrace
    pub fn native_unwind_info(&mut self, enable: bool) -> &mut Self {
        self.native_unwind_info = Some(enable);
        self
    }

    /// Configures whether execution of WebAssembly will "consume fuel" to
    /// either halt or yield execution as desired.
    ///
    /// This can be used to deterministically prevent infinitely-executing
    /// WebAssembly code by instrumenting generated code to consume fuel as it
    /// executes. When fuel runs out a trap is raised, however [`Store`] can be
    /// configured to yield execution periodically via
    /// [`crate::Store::fuel_async_yield_interval`].
    ///
    /// Note that a [`Store`] starts with no fuel, so if you enable this option
    /// you'll have to be sure to pour some fuel into [`Store`] before
    /// executing some code.
    ///
    /// By default this option is `false`.
    ///
    /// [`Store`]: crate::Store
    pub fn consume_fuel(&mut self, enable: bool) -> &mut Self {
        self.tunables.consume_fuel = Some(enable);
        self
    }

    /// Enables epoch-based interruption.
    ///
    /// When executing code in async mode, we sometimes want to
    /// implement a form of cooperative timeslicing: long-running Wasm
    /// guest code should periodically yield to the executor
    /// loop. This yielding could be implemented by using "fuel" (see
    /// [`consume_fuel`](Config::consume_fuel)). However, fuel
    /// instrumentation is somewhat expensive: it modifies the
    /// compiled form of the Wasm code so that it maintains a precise
    /// instruction count, frequently checking this count against the
    /// remaining fuel. If one does not need this precise count or
    /// deterministic interruptions, and only needs a periodic
    /// interrupt of some form, then It would be better to have a more
    /// lightweight mechanism.
    ///
    /// Epoch-based interruption is that mechanism. There is a global
    /// "epoch", which is a counter that divides time into arbitrary
    /// periods (or epochs). This counter lives on the
    /// [`Engine`](crate::Engine) and can be incremented by calling
    /// [`Engine::increment_epoch`](crate::Engine::increment_epoch).
    /// Epoch-based instrumentation works by setting a "deadline
    /// epoch". The compiled code knows the deadline, and at certain
    /// points, checks the current epoch against that deadline. It
    /// will yield if the deadline has been reached.
    ///
    /// The idea is that checking an infrequently-changing counter is
    /// cheaper than counting and frequently storing a precise metric
    /// (instructions executed) locally. The interruptions are not
    /// deterministic, but if the embedder increments the epoch in a
    /// periodic way (say, every regular timer tick by a thread or
    /// signal handler), then we can ensure that all async code will
    /// yield to the executor within a bounded time.
    ///
    /// The deadline check cannot be avoided by malicious wasm code. It is safe
    /// to use epoch deadlines to limit the execution time of untrusted
    /// code.
    ///
    /// The [`Store`](crate::Store) tracks the deadline, and controls
    /// what happens when the deadline is reached during
    /// execution. Several behaviors are possible:
    ///
    /// - Trap if code is executing when the epoch deadline is
    ///   met. See
    ///   [`Store::epoch_deadline_trap`](crate::Store::epoch_deadline_trap).
    ///
    /// - Call an arbitrary function. This function may chose to trap or
    ///   increment the epoch. See
    ///   [`Store::epoch_deadline_callback`](crate::Store::epoch_deadline_callback).
    ///
    /// - Yield to the executor loop, then resume when the future is
    ///   next polled. See
    ///   [`Store::epoch_deadline_async_yield_and_update`](crate::Store::epoch_deadline_async_yield_and_update).
    ///
    /// Trapping is the default. The yielding behaviour may be used for
    /// the timeslicing behavior described above.
    ///
    /// This feature is available with or without async support.
    /// However, without async support, the timeslicing behaviour is
    /// not available. This means epoch-based interruption can only
    /// serve as a simple external-interruption mechanism.
    ///
    /// An initial deadline must be set before executing code by calling
    /// [`Store::set_epoch_deadline`](crate::Store::set_epoch_deadline). If this
    /// deadline is not configured then wasm will immediately trap.
    ///
    /// ## When to use fuel vs. epochs
    ///
    /// In general, epoch-based interruption results in faster
    /// execution. This difference is sometimes significant: in some
    /// measurements, up to 2-3x. This is because epoch-based
    /// interruption does less work: it only watches for a global
    /// rarely-changing counter to increment, rather than keeping a
    /// local frequently-changing counter and comparing it to a
    /// deadline.
    ///
    /// Fuel, in contrast, should be used when *deterministic*
    /// yielding or trapping is needed. For example, if it is required
    /// that the same function call with the same starting state will
    /// always either complete or trap with an out-of-fuel error,
    /// deterministically, then fuel with a fixed bound should be
    /// used.
    ///
    /// # See Also
    ///
    /// - [`Engine::increment_epoch`](crate::Engine::increment_epoch)
    /// - [`Store::set_epoch_deadline`](crate::Store::set_epoch_deadline)
    /// - [`Store::epoch_deadline_trap`](crate::Store::epoch_deadline_trap)
    /// - [`Store::epoch_deadline_callback`](crate::Store::epoch_deadline_callback)
    /// - [`Store::epoch_deadline_async_yield_and_update`](crate::Store::epoch_deadline_async_yield_and_update)
    pub fn epoch_interruption(&mut self, enable: bool) -> &mut Self {
        self.tunables.epoch_interruption = Some(enable);
        self
    }

    /// Configures the maximum amount of stack space available for
    /// executing WebAssembly code.
    ///
    /// WebAssembly has well-defined semantics on stack overflow. This is
    /// intended to be a knob which can help configure how much stack space
    /// wasm execution is allowed to consume. Note that the number here is not
    /// super-precise, but rather wasm will take at most "pretty close to this
    /// much" stack space.
    ///
    /// If a wasm call (or series of nested wasm calls) take more stack space
    /// than the `size` specified then a stack overflow trap will be raised.
    ///
    /// Caveat: this knob only limits the stack space consumed by wasm code.
    /// More importantly, it does not ensure that this much stack space is
    /// available on the calling thread stack. Exhausting the thread stack
    /// typically leads to an **abort** of the process.
    ///
    /// Here are some examples of how that could happen:
    ///
    /// - Let's assume this option is set to 2 MiB and then a thread that has
    ///   a stack with 512 KiB left.
    ///
    ///   If wasm code consumes more than 512 KiB then the process will be aborted.
    ///
    /// - Assuming the same conditions, but this time wasm code does not consume
    ///   any stack but calls into a host function. The host function consumes
    ///   more than 512 KiB of stack space. The process will be aborted.
    ///
    /// There's another gotcha related to recursive calling into wasm: the stack
    /// space consumed by a host function is counted towards this limit. The
    /// host functions are not prevented from consuming more than this limit.
    /// However, if the host function that used more than this limit and called
    /// back into wasm, then the execution will trap immediatelly because of
    /// stack overflow.
    ///
    /// When the `async` feature is enabled, this value cannot exceed the
    /// `async_stack_size` option. Be careful not to set this value too close
    /// to `async_stack_size` as doing so may limit how much stack space
    /// is available for host functions.
    ///
    /// By default this option is 512 KiB.
    ///
    /// # Errors
    ///
    /// The `Engine::new` method will fail if the `size` specified here is
    /// either 0 or larger than the [`Config::async_stack_size`] configuration.
    pub fn max_wasm_stack(&mut self, size: usize) -> &mut Self {
        self.max_wasm_stack = size;
        self
    }

    /// Configures the size of the stacks used for asynchronous execution.
    ///
    /// This setting configures the size of the stacks that are allocated for
    /// asynchronous execution. The value cannot be less than `max_wasm_stack`.
    ///
    /// The amount of stack space guaranteed for host functions is
    /// `async_stack_size - max_wasm_stack`, so take care not to set these two values
    /// close to one another; doing so may cause host functions to overflow the
    /// stack and abort the process.
    ///
    /// By default this option is 2 MiB.
    ///
    /// # Errors
    ///
    /// The `Engine::new` method will fail if the value for this option is
    /// smaller than the [`Config::max_wasm_stack`] option.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn async_stack_size(&mut self, size: usize) -> &mut Self {
        self.async_stack_size = size;
        self
    }

    /// Configures whether the WebAssembly tail calls proposal will be enabled
    /// for compilation or not.
    ///
    /// The [WebAssembly tail calls proposal] introduces the `return_call` and
    /// `return_call_indirect` instructions. These instructions allow for Wasm
    /// programs to implement some recursive algorithms with *O(1)* stack space
    /// usage.
    ///
    /// This feature is disabled by default.
    ///
    /// [WebAssembly tail calls proposal]: https://github.com/WebAssembly/tail-call
    pub fn wasm_tail_call(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::TAIL_CALL, enable);
        self.tunables.tail_callable = Some(enable);
        self
    }

    /// Configures whether the WebAssembly [threads] proposal will be enabled
    /// for compilation.
    ///
    /// This feature gates items such as shared memories and atomic
    /// instructions. Note that the threads feature depends on the bulk memory
    /// feature, which is enabled by default. Additionally note that while the
    /// wasm feature is called "threads" it does not actually include the
    /// ability to spawn threads. Spawning threads is part of the [wasi-threads]
    /// proposal which is a separately gated feature in Wasmtime.
    ///
    /// Embeddings of Wasmtime are able to build their own custom threading
    /// scheme on top of the core wasm threads proposal, however.
    ///
    /// This is `true` by default.
    ///
    /// [threads]: https://github.com/webassembly/threads
    /// [wasi-threads]: https://github.com/webassembly/wasi-threads
    #[cfg(feature = "threads")]
    #[cfg_attr(docsrs, doc(cfg(feature = "threads")))]
    pub fn wasm_threads(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::THREADS, enable);
        self
    }

    /// Configures whether the [WebAssembly reference types proposal][proposal]
    /// will be enabled for compilation.
    ///
    /// This feature gates items such as the `externref` and `funcref` types as
    /// well as allowing a module to define multiple tables.
    ///
    /// Note that the reference types proposal depends on the bulk memory proposal.
    ///
    /// This feature is `true` by default.
    ///
    /// # Errors
    ///
    /// The validation of this feature are deferred until the engine is being built,
    /// and thus may cause `Engine::new` fail if the `bulk_memory` feature is disabled.
    ///
    /// [proposal]: https://github.com/webassembly/reference-types
    #[cfg(feature = "gc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gc")))]
    pub fn wasm_reference_types(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::REFERENCE_TYPES, enable);
        self
    }

    /// Configures whether the [WebAssembly function references
    /// proposal][proposal] will be enabled for compilation.
    ///
    /// This feature gates non-nullable reference types, function reference
    /// types, `call_ref`, `ref.func`, and non-nullable reference related
    /// instructions.
    ///
    /// Note that the function references proposal depends on the reference
    /// types proposal.
    ///
    /// This feature is `false` by default.
    ///
    /// [proposal]: https://github.com/WebAssembly/function-references
    #[cfg(feature = "gc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gc")))]
    pub fn wasm_function_references(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::FUNCTION_REFERENCES, enable);
        self
    }

    /// Configures whether the [WebAssembly Garbage Collection
    /// proposal][proposal] will be enabled for compilation.
    ///
    /// This feature gates `struct` and `array` type definitions and references,
    /// the `i31ref` type, and all related instructions.
    ///
    /// Note that the function references proposal depends on the typed function
    /// references proposal.
    ///
    /// This feature is `false` by default.
    ///
    /// **Warning: Wasmtime's implementation of the GC proposal is still in
    /// progress and generally not ready for primetime.**
    ///
    /// [proposal]: https://github.com/WebAssembly/gc
    #[cfg(feature = "gc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gc")))]
    pub fn wasm_gc(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::GC, enable);
        self
    }

    /// Configures whether the WebAssembly SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly SIMD proposal][proposal]. This feature gates items such
    /// as the `v128` type and all of its operators being in a module. Note that
    /// this does not enable the [relaxed simd proposal].
    ///
    /// On x86_64 platforms note that enabling this feature requires SSE 4.2 and
    /// below to be available on the target platform. Compilation will fail if
    /// the compile target does not include SSE 4.2.
    ///
    /// This is `true` by default.
    ///
    /// [proposal]: https://github.com/webassembly/simd
    /// [relaxed simd proposal]: https://github.com/WebAssembly/relaxed-simd
    pub fn wasm_simd(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::SIMD, enable);
        self
    }

    /// Configures whether the WebAssembly Relaxed SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The relaxed SIMD proposal adds new instructions to WebAssembly which,
    /// for some specific inputs, are allowed to produce different results on
    /// different hosts. More-or-less this proposal enables exposing
    /// platform-specific semantics of SIMD instructions in a controlled
    /// fashion to a WebAssembly program. From an embedder's perspective this
    /// means that WebAssembly programs may execute differently depending on
    /// whether the host is x86_64 or AArch64, for example.
    ///
    /// By default Wasmtime lowers relaxed SIMD instructions to the fastest
    /// lowering for the platform it's running on. This means that, by default,
    /// some relaxed SIMD instructions may have different results for the same
    /// inputs across x86_64 and AArch64. This behavior can be disabled through
    /// the [`Config::relaxed_simd_deterministic`] option which will force
    /// deterministic behavior across all platforms, as classified by the
    /// specification, at the cost of performance.
    ///
    /// This is `true` by default.
    ///
    /// [proposal]: https://github.com/webassembly/relaxed-simd
    pub fn wasm_relaxed_simd(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::RELAXED_SIMD, enable);
        self
    }

    /// This option can be used to control the behavior of the [relaxed SIMD
    /// proposal's][proposal] instructions.
    ///
    /// The relaxed SIMD proposal introduces instructions that are allowed to
    /// have different behavior on different architectures, primarily to afford
    /// an efficient implementation on all architectures. This means, however,
    /// that the same module may execute differently on one host than another,
    /// which typically is not otherwise the case. This option is provided to
    /// force Wasmtime to generate deterministic code for all relaxed simd
    /// instructions, at the cost of performance, for all architectures. When
    /// this option is enabled then the deterministic behavior of all
    /// instructions in the relaxed SIMD proposal is selected.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/relaxed-simd
    pub fn relaxed_simd_deterministic(&mut self, enable: bool) -> &mut Self {
        self.tunables.relaxed_simd_deterministic = Some(enable);
        self
    }

    /// Configures whether the [WebAssembly bulk memory operations
    /// proposal][proposal] will be enabled for compilation.
    ///
    /// This feature gates items such as the `memory.copy` instruction, passive
    /// data/table segments, etc, being in a module.
    ///
    /// This is `true` by default.
    ///
    /// Feature `reference_types`, which is also `true` by default, requires
    /// this feature to be enabled. Thus disabling this feature must also disable
    /// `reference_types` as well using [`wasm_reference_types`](crate::Config::wasm_reference_types).
    ///
    /// # Errors
    ///
    /// Disabling this feature without disabling `reference_types` will cause
    /// `Engine::new` to fail.
    ///
    /// [proposal]: https://github.com/webassembly/bulk-memory-operations
    pub fn wasm_bulk_memory(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::BULK_MEMORY, enable);
        self
    }

    /// Configures whether the WebAssembly multi-value [proposal] will
    /// be enabled for compilation.
    ///
    /// This feature gates functions and blocks returning multiple values in a
    /// module, for example.
    ///
    /// This is `true` by default.
    ///
    /// [proposal]: https://github.com/webassembly/multi-value
    pub fn wasm_multi_value(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::MULTI_VALUE, enable);
        self
    }

    /// Configures whether the WebAssembly multi-memory [proposal] will
    /// be enabled for compilation.
    ///
    /// This feature gates modules having more than one linear memory
    /// declaration or import.
    ///
    /// This is `true` by default.
    ///
    /// [proposal]: https://github.com/webassembly/multi-memory
    pub fn wasm_multi_memory(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::MULTI_MEMORY, enable);
        self
    }

    /// Configures whether the WebAssembly memory64 [proposal] will
    /// be enabled for compilation.
    ///
    /// Note that this the upstream specification is not finalized and Wasmtime
    /// may also have bugs for this feature since it hasn't been exercised
    /// much.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/memory64
    pub fn wasm_memory64(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::MEMORY64, enable);
        self
    }

    /// Configures whether the WebAssembly component-model [proposal] will
    /// be enabled for compilation.
    ///
    /// Note that this feature is a work-in-progress and is incomplete.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/component-model
    #[cfg(feature = "component-model")]
    pub fn wasm_component_model(&mut self, enable: bool) -> &mut Self {
        self.features.set(WasmFeatures::COMPONENT_MODEL, enable);
        self
    }

    /// Configures whether we enable the "indirect call cache" optimization.
    ///
    /// This feature adds, for each `call_indirect` instruction in a
    /// Wasm module (i.e., a function-pointer call in guest code), a
    /// one-entry cache that speeds up the translation from a table
    /// index to the actual machine code. By default, the VM's
    /// implementation of this translation requires several
    /// indirections and checks (table bounds-check, function
    /// signature-check, table lazy-initialization logic). The intent
    /// of this feature is to speed up indirect calls substantially
    /// when they are repeated frequently in hot code.
    ///
    /// While it accelerates repeated calls, this feature has the
    /// potential to slow down instantiation slightly, because it adds
    /// additional state (the cache storage -- usually 16 bytes per
    /// `call_indirect` instruction for each instance) that has to be
    /// initialized. In practice, we have not seen
    /// measurable/statistically-significant impact from this, though.
    ///
    /// Until we have further experience with this feature, it will
    /// remain off: it is `false` by default.
    pub fn cache_call_indirects(&mut self, enable: bool) -> &mut Self {
        self.tunables.cache_call_indirects = Some(enable);
        self
    }

    /// Configures which compilation strategy will be used for wasm modules.
    ///
    /// This method can be used to configure which compiler is used for wasm
    /// modules, and for more documentation consult the [`Strategy`] enumeration
    /// and its documentation.
    ///
    /// The default value for this is `Strategy::Auto`.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn strategy(&mut self, strategy: Strategy) -> &mut Self {
        self.compiler_config.strategy = strategy;
        self
    }

    /// Creates a default profiler based on the profiling strategy chosen.
    ///
    /// Profiler creation calls the type's default initializer where the purpose is
    /// really just to put in place the type used for profiling.
    ///
    /// Some [`ProfilingStrategy`] require specific platforms or particular feature
    /// to be enabled, such as `ProfilingStrategy::JitDump` requires the `jitdump`
    /// feature.
    ///
    /// # Errors
    ///
    /// The validation of this field is deferred until the engine is being built, and thus may
    /// cause `Engine::new` fail if the required feature is disabled, or the platform is not
    /// supported.
    pub fn profiler(&mut self, profile: ProfilingStrategy) -> &mut Self {
        self.profiling_strategy = profile;
        self
    }

    /// Configures whether the debug verifier of Cranelift is enabled or not.
    ///
    /// When Cranelift is used as a code generation backend this will configure
    /// it to have the `enable_verifier` flag which will enable a number of debug
    /// checks inside of Cranelift. This is largely only useful for the
    /// developers of wasmtime itself.
    ///
    /// The default value for this is `false`
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn cranelift_debug_verifier(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler_config
            .settings
            .insert("enable_verifier".to_string(), val.to_string());
        self
    }

    /// Configures the Cranelift code generator optimization level.
    ///
    /// When the Cranelift code generator is used you can configure the
    /// optimization level used for generated code in a few various ways. For
    /// more information see the documentation of [`OptLevel`].
    ///
    /// The default value for this is `OptLevel::None`.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn cranelift_opt_level(&mut self, level: OptLevel) -> &mut Self {
        let val = match level {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        };
        self.compiler_config
            .settings
            .insert("opt_level".to_string(), val.to_string());
        self
    }

    /// Configures whether Cranelift should perform a NaN-canonicalization pass.
    ///
    /// When Cranelift is used as a code generation backend this will configure
    /// it to replace NaNs with a single canonical value. This is useful for
    /// users requiring entirely deterministic WebAssembly computation.  This is
    /// not required by the WebAssembly spec, so it is not enabled by default.
    ///
    /// Note that this option affects not only WebAssembly's `f32` and `f64`
    /// types but additionally the `v128` type. This option will cause
    /// operations using any of these types to have extra checks placed after
    /// them to normalize NaN values as needed.
    ///
    /// The default value for this is `false`
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn cranelift_nan_canonicalization(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler_config
            .settings
            .insert("enable_nan_canonicalization".to_string(), val.to_string());
        self
    }

    /// Controls whether proof-carrying code (PCC) is used to validate
    /// lowering of Wasm sandbox checks.
    ///
    /// Proof-carrying code carries "facts" about program values from
    /// the IR all the way to machine code, and checks those facts
    /// against known machine-instruction semantics. This guards
    /// against bugs in instruction lowering that might create holes
    /// in the Wasm sandbox.
    ///
    /// PCC is designed to be fast: it does not require complex
    /// solvers or logic engines to verify, but only a linear pass
    /// over a trail of "breadcrumbs" or facts at each intermediate
    /// value. Thus, it is appropriate to enable in production.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub fn cranelift_pcc(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler_config
            .settings
            .insert("enable_pcc".to_string(), val.to_string());
        self
    }

    /// Allows setting a Cranelift boolean flag or preset. This allows
    /// fine-tuning of Cranelift settings.
    ///
    /// Since Cranelift flags may be unstable, this method should not be considered to be stable
    /// either; other `Config` functions should be preferred for stability.
    ///
    /// # Safety
    ///
    /// This is marked as unsafe, because setting the wrong flag might break invariants,
    /// resulting in execution hazards.
    ///
    /// # Errors
    ///
    /// The validation of the flags are deferred until the engine is being built, and thus may
    /// cause `Engine::new` fail if the flag's name does not exist, or the value is not appropriate
    /// for the flag type.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub unsafe fn cranelift_flag_enable(&mut self, flag: &str) -> &mut Self {
        self.compiler_config.flags.insert(flag.to_string());
        self
    }

    /// Allows settings another Cranelift flag defined by a flag name and value. This allows
    /// fine-tuning of Cranelift settings.
    ///
    /// Since Cranelift flags may be unstable, this method should not be considered to be stable
    /// either; other `Config` functions should be preferred for stability.
    ///
    /// # Safety
    ///
    /// This is marked as unsafe, because setting the wrong flag might break invariants,
    /// resulting in execution hazards.
    ///
    /// # Errors
    ///
    /// The validation of the flags are deferred until the engine is being built, and thus may
    /// cause `Engine::new` fail if the flag's name does not exist, or incompatible with other
    /// settings.
    ///
    /// For example, feature `wasm_backtrace` will set `unwind_info` to `true`, but if it's
    /// manually set to false then it will fail.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "cranelift", feature = "winch"))))]
    pub unsafe fn cranelift_flag_set(&mut self, name: &str, value: &str) -> &mut Self {
        self.compiler_config
            .settings
            .insert(name.to_string(), value.to_string());
        self
    }

    /// Loads cache configuration specified at `path`.
    ///
    /// This method will read the file specified by `path` on the filesystem and
    /// attempt to load cache configuration from it. This method can also fail
    /// due to I/O errors, misconfiguration, syntax errors, etc. For expected
    /// syntax in the configuration file see the [documentation online][docs].
    ///
    /// By default cache configuration is not enabled or loaded.
    ///
    /// This method is only available when the `cache` feature of this crate is
    /// enabled.
    ///
    /// # Errors
    ///
    /// This method can fail due to any error that happens when loading the file
    /// pointed to by `path` and attempting to load the cache configuration.
    ///
    /// [docs]: https://bytecodealliance.github.io/wasmtime/cli-cache.html
    #[cfg(feature = "cache")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
    pub fn cache_config_load(&mut self, path: impl AsRef<Path>) -> Result<&mut Self> {
        self.cache_config = CacheConfig::from_file(Some(path.as_ref()))?;
        Ok(self)
    }

    /// Disable caching.
    ///
    /// Every call to [`Module::new(my_wasm)`][crate::Module::new] will
    /// recompile `my_wasm`, even when it is unchanged.
    ///
    /// By default, new configs do not have caching enabled. This method is only
    /// useful for disabling a previous cache configuration.
    ///
    /// This method is only available when the `cache` feature of this crate is
    /// enabled.
    #[cfg(feature = "cache")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
    pub fn disable_cache(&mut self) -> &mut Self {
        self.cache_config = CacheConfig::new_cache_disabled();
        self
    }

    /// Loads cache configuration from the system default path.
    ///
    /// This commit is the same as [`Config::cache_config_load`] except that it
    /// does not take a path argument and instead loads the default
    /// configuration present on the system. This is located, for example, on
    /// Unix at `$HOME/.config/wasmtime/config.toml` and is typically created
    /// with the `wasmtime config new` command.
    ///
    /// By default cache configuration is not enabled or loaded.
    ///
    /// This method is only available when the `cache` feature of this crate is
    /// enabled.
    ///
    /// # Errors
    ///
    /// This method can fail due to any error that happens when loading the
    /// default system configuration. Note that it is not an error if the
    /// default config file does not exist, in which case the default settings
    /// for an enabled cache are applied.
    ///
    /// [docs]: https://bytecodealliance.github.io/wasmtime/cli-cache.html
    #[cfg(feature = "cache")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cache")))]
    pub fn cache_config_load_default(&mut self) -> Result<&mut Self> {
        self.cache_config = CacheConfig::from_file(None)?;
        Ok(self)
    }

    /// Sets a custom memory creator.
    ///
    /// Custom memory creators are used when creating host `Memory` objects or when
    /// creating instance linear memories for the on-demand instance allocation strategy.
    #[cfg(feature = "runtime")]
    pub fn with_host_memory(&mut self, mem_creator: Arc<dyn MemoryCreator>) -> &mut Self {
        self.mem_creator = Some(Arc::new(MemoryCreatorProxy(mem_creator)));
        self
    }

    /// Sets a custom stack creator.
    ///
    /// Custom memory creators are used when creating creating async instance stacks for
    /// the on-demand instance allocation strategy.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn with_host_stack(&mut self, stack_creator: Arc<dyn StackCreator>) -> &mut Self {
        self.stack_creator = Some(Arc::new(StackCreatorProxy(stack_creator)));
        self
    }

    /// Sets the instance allocation strategy to use.
    ///
    /// When using the pooling instance allocation strategy, all linear memories
    /// will be created as "static" and the
    /// [`Config::static_memory_maximum_size`] and
    /// [`Config::static_memory_guard_size`] options will be used to configure
    /// the virtual memory allocations of linear memories.
    pub fn allocation_strategy(&mut self, strategy: InstanceAllocationStrategy) -> &mut Self {
        self.allocation_strategy = strategy;
        self
    }

    /// Configures the maximum size, in bytes, where a linear memory is
    /// considered static, above which it'll be considered dynamic.
    ///
    /// > Note: this value has important performance ramifications, be sure to
    /// > understand what this value does before tweaking it and benchmarking.
    ///
    /// This function configures the threshold for wasm memories whether they're
    /// implemented as a dynamically relocatable chunk of memory or a statically
    /// located chunk of memory. The `max_size` parameter here is the size, in
    /// bytes, where if the maximum size of a linear memory is below `max_size`
    /// then it will be statically allocated with enough space to never have to
    /// move. If the maximum size of a linear memory is larger than `max_size`
    /// then wasm memory will be dynamically located and may move in memory
    /// through growth operations.
    ///
    /// Specifying a `max_size` of 0 means that all memories will be dynamic and
    /// may be relocated through `memory.grow`. Also note that if any wasm
    /// memory's maximum size is below `max_size` then it will still reserve
    /// `max_size` bytes in the virtual memory space.
    ///
    /// ## Static vs Dynamic Memory
    ///
    /// Linear memories represent contiguous arrays of bytes, but they can also
    /// be grown through the API and wasm instructions. When memory is grown if
    /// space hasn't been preallocated then growth may involve relocating the
    /// base pointer in memory. Memories in Wasmtime are classified in two
    /// different ways:
    ///
    /// * **static** - these memories preallocate all space necessary they'll
    ///   ever need, meaning that the base pointer of these memories is never
    ///   moved. Static memories may take more virtual memory space because of
    ///   pre-reserving space for memories.
    ///
    /// * **dynamic** - these memories are not preallocated and may move during
    ///   growth operations. Dynamic memories consume less virtual memory space
    ///   because they don't need to preallocate space for future growth.
    ///
    /// Static memories can be optimized better in JIT code because once the
    /// base address is loaded in a function it's known that we never need to
    /// reload it because it never changes, `memory.grow` is generally a pretty
    /// fast operation because the wasm memory is never relocated, and under
    /// some conditions bounds checks can be elided on memory accesses.
    ///
    /// Dynamic memories can't be quite as heavily optimized because the base
    /// address may need to be reloaded more often, they may require relocating
    /// lots of data on `memory.grow`, and dynamic memories require
    /// unconditional bounds checks on all memory accesses.
    ///
    /// ## Should you use static or dynamic memory?
    ///
    /// In general you probably don't need to change the value of this property.
    /// The defaults here are optimized for each target platform to consume a
    /// reasonable amount of physical memory while also generating speedy
    /// machine code.
    ///
    /// One of the main reasons you may want to configure this today is if your
    /// environment can't reserve virtual memory space for each wasm linear
    /// memory. On 64-bit platforms wasm memories require a 6GB reservation by
    /// default, and system limits may prevent this in some scenarios. In this
    /// case you may wish to force memories to be allocated dynamically meaning
    /// that the virtual memory footprint of creating a wasm memory should be
    /// exactly what's used by the wasm itself.
    ///
    /// For 32-bit memories a static memory must contain at least 4GB of
    /// reserved address space plus a guard page to elide any bounds checks at
    /// all. Smaller static memories will use similar bounds checks as dynamic
    /// memories.
    ///
    /// ## Default
    ///
    /// The default value for this property depends on the host platform. For
    /// 64-bit platforms there's lots of address space available, so the default
    /// configured here is 4GB. WebAssembly linear memories currently max out at
    /// 4GB which means that on 64-bit platforms Wasmtime by default always uses
    /// a static memory. This, coupled with a sufficiently sized guard region,
    /// should produce the fastest JIT code on 64-bit platforms, but does
    /// require a large address space reservation for each wasm memory.
    ///
    /// For 32-bit platforms this value defaults to 1GB. This means that wasm
    /// memories whose maximum size is less than 1GB will be allocated
    /// statically, otherwise they'll be considered dynamic.
    ///
    /// ## Static Memory and Pooled Instance Allocation
    ///
    /// When using the pooling instance allocator memories are considered to
    /// always be static memories, they are never dynamic. This setting
    /// configures the size of linear memory to reserve for each memory in the
    /// pooling allocator.
    ///
    /// Note that the pooling allocator can reduce the amount of memory needed
    /// for pooling allocation by using memory protection; see
    /// `PoolingAllocatorConfig::memory_protection_keys` for details.
    pub fn static_memory_maximum_size(&mut self, max_size: u64) -> &mut Self {
        let max_pages = max_size / u64::from(wasmtime_environ::WASM_PAGE_SIZE);
        self.tunables.static_memory_bound = Some(max_pages);
        self
    }

    /// Indicates that the "static" style of memory should always be used.
    ///
    /// This configuration option enables selecting the "static" option for all
    /// linear memories created within this `Config`. This means that all
    /// memories will be allocated up-front and will never move. Additionally
    /// this means that all memories are synthetically limited by the
    /// [`Config::static_memory_maximum_size`] option, regardless of what the
    /// actual maximum size is on the memory's original type.
    ///
    /// For the difference between static and dynamic memories, see the
    /// [`Config::static_memory_maximum_size`].
    pub fn static_memory_forced(&mut self, force: bool) -> &mut Self {
        self.tunables.static_memory_bound_is_maximum = Some(force);
        self
    }

    /// Configures the size, in bytes, of the guard region used at the end of a
    /// static memory's address space reservation.
    ///
    /// > Note: this value has important performance ramifications, be sure to
    /// > understand what this value does before tweaking it and benchmarking.
    ///
    /// All WebAssembly loads/stores are bounds-checked and generate a trap if
    /// they're out-of-bounds. Loads and stores are often very performance
    /// critical, so we want the bounds check to be as fast as possible!
    /// Accelerating these memory accesses is the motivation for a guard after a
    /// memory allocation.
    ///
    /// Memories (both static and dynamic) can be configured with a guard at the
    /// end of them which consists of unmapped virtual memory. This unmapped
    /// memory will trigger a memory access violation (e.g. segfault) if
    /// accessed. This allows JIT code to elide bounds checks if it can prove
    /// that an access, if out of bounds, would hit the guard region. This means
    /// that having such a guard of unmapped memory can remove the need for
    /// bounds checks in JIT code.
    ///
    /// For the difference between static and dynamic memories, see the
    /// [`Config::static_memory_maximum_size`].
    ///
    /// ## How big should the guard be?
    ///
    /// In general, like with configuring `static_memory_maximum_size`, you
    /// probably don't want to change this value from the defaults. Otherwise,
    /// though, the size of the guard region affects the number of bounds checks
    /// needed for generated wasm code. More specifically, loads/stores with
    /// immediate offsets will generate bounds checks based on how big the guard
    /// page is.
    ///
    /// For 32-bit wasm memories a 4GB static memory is required to even start
    /// removing bounds checks. A 4GB guard size will guarantee that the module
    /// has zero bounds checks for memory accesses. A 2GB guard size will
    /// eliminate all bounds checks with an immediate offset less than 2GB. A
    /// guard size of zero means that all memory accesses will still have bounds
    /// checks.
    ///
    /// ## Default
    ///
    /// The default value for this property is 2GB on 64-bit platforms. This
    /// allows eliminating almost all bounds checks on loads/stores with an
    /// immediate offset of less than 2GB. On 32-bit platforms this defaults to
    /// 64KB.
    ///
    /// ## Errors
    ///
    /// The `Engine::new` method will return an error if this option is smaller
    /// than the value configured for [`Config::dynamic_memory_guard_size`].
    pub fn static_memory_guard_size(&mut self, guard_size: u64) -> &mut Self {
        let guard_size = round_up_to_pages(guard_size);
        self.tunables.static_memory_offset_guard_size = Some(guard_size);
        self
    }

    /// Configures the size, in bytes, of the guard region used at the end of a
    /// dynamic memory's address space reservation.
    ///
    /// For the difference between static and dynamic memories, see the
    /// [`Config::static_memory_maximum_size`]
    ///
    /// For more information about what a guard is, see the documentation on
    /// [`Config::static_memory_guard_size`].
    ///
    /// Note that the size of the guard region for dynamic memories is not super
    /// critical for performance. Making it reasonably-sized can improve
    /// generated code slightly, but for maximum performance you'll want to lean
    /// towards static memories rather than dynamic anyway.
    ///
    /// Also note that the dynamic memory guard size must be smaller than the
    /// static memory guard size, so if a large dynamic memory guard is
    /// specified then the static memory guard size will also be automatically
    /// increased.
    ///
    /// ## Default
    ///
    /// This value defaults to 64KB.
    ///
    /// ## Errors
    ///
    /// The `Engine::new` method will return an error if this option is larger
    /// than the value configured for [`Config::static_memory_guard_size`].
    pub fn dynamic_memory_guard_size(&mut self, guard_size: u64) -> &mut Self {
        let guard_size = round_up_to_pages(guard_size);
        self.tunables.dynamic_memory_offset_guard_size = Some(guard_size);
        self
    }

    /// Configures the size, in bytes, of the extra virtual memory space
    /// reserved after a "dynamic" memory for growing into.
    ///
    /// For the difference between static and dynamic memories, see the
    /// [`Config::static_memory_maximum_size`]
    ///
    /// Dynamic memories can be relocated in the process's virtual address space
    /// on growth and do not always reserve their entire space up-front. This
    /// means that a growth of the memory may require movement in the address
    /// space, which in the worst case can copy a large number of bytes from one
    /// region to another.
    ///
    /// This setting configures how many bytes are reserved after the initial
    /// reservation for a dynamic memory for growing into. A value of 0 here
    /// means that no extra bytes are reserved and all calls to `memory.grow`
    /// will need to relocate the wasm linear memory (copying all the bytes). A
    /// value of 1 megabyte, however, means that `memory.grow` can allocate up
    /// to a megabyte of extra memory before the memory needs to be moved in
    /// linear memory.
    ///
    /// Note that this is a currently simple heuristic for optimizing the growth
    /// of dynamic memories, primarily implemented for the memory64 proposal
    /// where all memories are currently "dynamic". This is unlikely to be a
    /// one-size-fits-all style approach and if you're an embedder running into
    /// issues with dynamic memories and growth and are interested in having
    /// other growth strategies available here please feel free to [open an
    /// issue on the Wasmtime repository][issue]!
    ///
    /// [issue]: https://github.com/bytecodealliance/wasmtime/issues/ne
    ///
    /// ## Default
    ///
    /// For 64-bit platforms this defaults to 2GB, and for 32-bit platforms this
    /// defaults to 1MB.
    pub fn dynamic_memory_reserved_for_growth(&mut self, reserved: u64) -> &mut Self {
        self.tunables.dynamic_memory_growth_reserve = Some(round_up_to_pages(reserved));
        self
    }

    /// Indicates whether a guard region is present before allocations of
    /// linear memory.
    ///
    /// Guard regions before linear memories are never used during normal
    /// operation of WebAssembly modules, even if they have out-of-bounds
    /// loads. The only purpose for a preceding guard region in linear memory
    /// is extra protection against possible bugs in code generators like
    /// Cranelift. This setting does not affect performance in any way, but will
    /// result in larger virtual memory reservations for linear memories (it
    /// won't actually ever use more memory, just use more of the address
    /// space).
    ///
    /// The size of the guard region before linear memory is the same as the
    /// guard size that comes after linear memory, which is configured by
    /// [`Config::static_memory_guard_size`] and
    /// [`Config::dynamic_memory_guard_size`].
    ///
    /// ## Default
    ///
    /// This value defaults to `true`.
    pub fn guard_before_linear_memory(&mut self, guard: bool) -> &mut Self {
        self.tunables.guard_before_linear_memory = Some(guard);
        self
    }

    /// Configure the version information used in serialized and deserialzied [`crate::Module`]s.
    /// This effects the behavior of [`crate::Module::serialize()`], as well as
    /// [`crate::Module::deserialize()`] and related functions.
    ///
    /// The default strategy is to use the wasmtime crate's Cargo package version.
    pub fn module_version(&mut self, strategy: ModuleVersionStrategy) -> Result<&mut Self> {
        match strategy {
            // This case requires special precondition for assertion in SerializedModule::to_bytes
            ModuleVersionStrategy::Custom(ref v) => {
                if v.as_bytes().len() > 255 {
                    bail!("custom module version cannot be more than 255 bytes: {}", v);
                }
            }
            _ => {}
        }
        self.module_version = strategy;
        Ok(self)
    }

    /// Configure wether wasmtime should compile a module using multiple
    /// threads.
    ///
    /// Disabling this will result in a single thread being used to compile
    /// the wasm bytecode.
    ///
    /// By default parallel compilation is enabled.
    #[cfg(feature = "parallel-compilation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "parallel-compilation")))]
    pub fn parallel_compilation(&mut self, parallel: bool) -> &mut Self {
        self.parallel_compilation = parallel;
        self
    }

    /// Configures whether compiled artifacts will contain information to map
    /// native program addresses back to the original wasm module.
    ///
    /// This configuration option is `true` by default and, if enabled,
    /// generates the appropriate tables in compiled modules to map from native
    /// address back to wasm source addresses. This is used for displaying wasm
    /// program counters in backtraces as well as generating filenames/line
    /// numbers if so configured as well (and the original wasm module has DWARF
    /// debugging information present).
    pub fn generate_address_map(&mut self, generate: bool) -> &mut Self {
        self.tunables.generate_address_map = Some(generate);
        self
    }

    /// Configures whether copy-on-write memory-mapped data is used to
    /// initialize a linear memory.
    ///
    /// Initializing linear memory via a copy-on-write mapping can drastically
    /// improve instantiation costs of a WebAssembly module because copying
    /// memory is deferred. Additionally if a page of memory is only ever read
    /// from WebAssembly and never written too then the same underlying page of
    /// data will be reused between all instantiations of a module meaning that
    /// if a module is instantiated many times this can lower the overall memory
    /// required needed to run that module.
    ///
    /// The main disadvantage of copy-on-write initialization, however, is that
    /// it may be possible for highly-parallel scenarios to be less scalable. If
    /// a page is read initially by a WebAssembly module then that page will be
    /// mapped to a read-only copy shared between all WebAssembly instances. If
    /// the same page is then written, however, then a private copy is created
    /// and swapped out from the read-only version. This also requires an [IPI],
    /// however, which can be a significant bottleneck in high-parallelism
    /// situations.
    ///
    /// This feature is only applicable when a WebAssembly module meets specific
    /// criteria to be initialized in this fashion, such as:
    ///
    /// * Only memories defined in the module can be initialized this way.
    /// * Data segments for memory must use statically known offsets.
    /// * Data segments for memory must all be in-bounds.
    ///
    /// Modules which do not meet these criteria will fall back to
    /// initialization of linear memory based on copying memory.
    ///
    /// This feature of Wasmtime is also platform-specific:
    ///
    /// * Linux - this feature is supported for all instances of [`Module`].
    ///   Modules backed by an existing mmap (such as those created by
    ///   [`Module::deserialize_file`]) will reuse that mmap to cow-initialize
    ///   memory. Other instance of [`Module`] may use the `memfd_create`
    ///   syscall to create an initialization image to `mmap`.
    /// * Unix (not Linux) - this feature is only supported when loading modules
    ///   from a precompiled file via [`Module::deserialize_file`] where there
    ///   is a file descriptor to use to map data into the process. Note that
    ///   the module must have been compiled with this setting enabled as well.
    /// * Windows - there is no support for this feature at this time. Memory
    ///   initialization will always copy bytes.
    ///
    /// By default this option is enabled.
    ///
    /// [`Module::deserialize_file`]: crate::Module::deserialize_file
    /// [`Module`]: crate::Module
    /// [IPI]: https://en.wikipedia.org/wiki/Inter-processor_interrupt
    pub fn memory_init_cow(&mut self, enable: bool) -> &mut Self {
        self.memory_init_cow = enable;
        self
    }

    /// A configuration option to force the usage of `memfd_create` on Linux to
    /// be used as the backing source for a module's initial memory image.
    ///
    /// When [`Config::memory_init_cow`] is enabled, which is enabled by
    /// default, module memory initialization images are taken from a module's
    /// original mmap if possible. If a precompiled module was loaded from disk
    /// this means that the disk's file is used as an mmap source for the
    /// initial linear memory contents. This option can be used to force, on
    /// Linux, that instead of using the original file on disk a new in-memory
    /// file is created with `memfd_create` to hold the contents of the initial
    /// image.
    ///
    /// This option can be used to avoid possibly loading the contents of memory
    /// from disk through a page fault. Instead with `memfd_create` the contents
    /// of memory are always in RAM, meaning that even page faults which
    /// initially populate a wasm linear memory will only work with RAM instead
    /// of ever hitting the disk that the original precompiled module is stored
    /// on.
    ///
    /// This option is disabled by default.
    pub fn force_memory_init_memfd(&mut self, enable: bool) -> &mut Self {
        self.force_memory_init_memfd = enable;
        self
    }

    /// Configures whether or not a coredump should be generated and attached to
    /// the anyhow::Error when a trap is raised.
    ///
    /// This option is disabled by default.
    #[cfg(feature = "coredump")]
    #[cfg_attr(docsrs, doc(cfg(feature = "coredump")))]
    pub fn coredump_on_trap(&mut self, enable: bool) -> &mut Self {
        self.coredump_on_trap = enable;
        self
    }

    /// Enables memory error checking for wasm programs.
    ///
    /// This option is disabled by default.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn wmemcheck(&mut self, enable: bool) -> &mut Self {
        self.wmemcheck = enable;
        self.compiler_config.wmemcheck = enable;
        self
    }

    /// Configures the "guaranteed dense image size" for copy-on-write
    /// initialized memories.
    ///
    /// When using the [`Config::memory_init_cow`] feature to initialize memory
    /// efficiently (which is enabled by default), compiled modules contain an
    /// image of the module's initial heap. If the module has a fairly sparse
    /// initial heap, with just a few data segments at very different offsets,
    /// this could result in a large region of zero bytes in the image. In
    /// other words, it's not very memory-efficient.
    ///
    /// We normally use a heuristic to avoid this: if less than half
    /// of the initialized range (first non-zero to last non-zero
    /// byte) of any memory in the module has pages with nonzero
    /// bytes, then we avoid creating a memory image for the entire module.
    ///
    /// However, if the embedder always needs the instantiation-time efficiency
    /// of copy-on-write initialization, and is otherwise carefully controlling
    /// parameters of the modules (for example, by limiting the maximum heap
    /// size of the modules), then it may be desirable to ensure a memory image
    /// is created even if this could go against the heuristic above. Thus, we
    /// add another condition: there is a size of initialized data region up to
    /// which we *always* allow a memory image. The embedder can set this to a
    /// known maximum heap size if they desire to always get the benefits of
    /// copy-on-write images.
    ///
    /// In the future we may implement a "best of both worlds"
    /// solution where we have a dense image up to some limit, and
    /// then support a sparse list of initializers beyond that; this
    /// would get most of the benefit of copy-on-write and pay the incremental
    /// cost of eager initialization only for those bits of memory
    /// that are out-of-bounds. However, for now, an embedder desiring
    /// fast instantiation should ensure that this setting is as large
    /// as the maximum module initial memory content size.
    ///
    /// By default this value is 16 MiB.
    pub fn memory_guaranteed_dense_image_size(&mut self, size_in_bytes: u64) -> &mut Self {
        self.memory_guaranteed_dense_image_size = size_in_bytes;
        self
    }

    pub(crate) fn validate(&self) -> Result<Tunables> {
        if self.features.contains(WasmFeatures::REFERENCE_TYPES)
            && !self.features.contains(WasmFeatures::BULK_MEMORY)
        {
            bail!("feature 'reference_types' requires 'bulk_memory' to be enabled");
        }
        if self.features.contains(WasmFeatures::THREADS)
            && !self.features.contains(WasmFeatures::BULK_MEMORY)
        {
            bail!("feature 'threads' requires 'bulk_memory' to be enabled");
        }
        if self.features.contains(WasmFeatures::FUNCTION_REFERENCES)
            && !self.features.contains(WasmFeatures::REFERENCE_TYPES)
        {
            bail!("feature 'function_references' requires 'reference_types' to be enabled");
        }
        if self.features.contains(WasmFeatures::GC)
            && !self.features.contains(WasmFeatures::FUNCTION_REFERENCES)
        {
            bail!("feature 'gc' requires 'function_references' to be enabled");
        }
        #[cfg(feature = "async")]
        if self.async_support && self.max_wasm_stack > self.async_stack_size {
            bail!("max_wasm_stack size cannot exceed the async_stack_size");
        }
        if self.max_wasm_stack == 0 {
            bail!("max_wasm_stack size cannot be zero");
        }
        #[cfg(not(feature = "wmemcheck"))]
        if self.wmemcheck {
            bail!("wmemcheck (memory checker) was requested but is not enabled in this build");
        }

        #[cfg(not(any(feature = "cranelift", feature = "winch")))]
        let mut tunables = Tunables::default_host();
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        let mut tunables = match &self.compiler_config.target.as_ref() {
            Some(target) => Tunables::default_for_target(target)?,
            None => Tunables::default_host(),
        };

        macro_rules! set_fields {
            ($($field:ident)*) => (
                let ConfigTunables {
                    $($field,)*
                } = &self.tunables;

                $(
                    if let Some(e) = $field {
                        tunables.$field = *e;
                    }
                )*
            )
        }

        set_fields! {
            static_memory_bound
            static_memory_offset_guard_size
            dynamic_memory_offset_guard_size
            dynamic_memory_growth_reserve
            generate_native_debuginfo
            parse_wasm_debuginfo
            consume_fuel
            epoch_interruption
            static_memory_bound_is_maximum
            guard_before_linear_memory
            generate_address_map
            debug_adapter_modules
            relaxed_simd_deterministic
            tail_callable
            cache_call_indirects
        }

        // If we're going to compile with winch, we must use the winch calling convention.
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        {
            tunables.winch_callable = match self.compiler_config.strategy {
                Strategy::Auto => !cfg!(feature = "cranelift") && cfg!(feature = "winch"),
                Strategy::Cranelift => false,
                Strategy::Winch => true,
            };

            if tunables.winch_callable && tunables.tail_callable {
                bail!("Winch does not support the WebAssembly tail call proposal");
            }
        }

        if tunables.static_memory_offset_guard_size < tunables.dynamic_memory_offset_guard_size {
            bail!("static memory guard size cannot be smaller than dynamic memory guard size");
        }

        Ok(tunables)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn build_allocator(
        &self,
        tunables: &Tunables,
    ) -> Result<Box<dyn InstanceAllocator + Send + Sync>> {
        #[cfg(feature = "async")]
        let stack_size = self.async_stack_size;

        #[cfg(not(feature = "async"))]
        let stack_size = 0;

        let _ = tunables;

        match &self.allocation_strategy {
            InstanceAllocationStrategy::OnDemand => {
                #[allow(unused_mut)]
                let mut allocator = Box::new(OnDemandInstanceAllocator::new(
                    self.mem_creator.clone(),
                    stack_size,
                ));
                #[cfg(feature = "async")]
                if let Some(stack_creator) = &self.stack_creator {
                    allocator.set_stack_creator(stack_creator.clone());
                }
                Ok(allocator)
            }
            #[cfg(feature = "pooling-allocator")]
            InstanceAllocationStrategy::Pooling(config) => {
                let mut config = config.config;
                config.stack_size = stack_size;
                Ok(Box::new(wasmtime_runtime::PoolingInstanceAllocator::new(
                    &config, tunables,
                )?))
            }
        }
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn build_gc_runtime(&self) -> Result<Arc<dyn GcRuntime>> {
        Ok(Arc::new(wasmtime_runtime::default_gc_runtime()) as Arc<dyn GcRuntime>)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn build_profiler(&self) -> Result<Box<dyn ProfilingAgent>> {
        Ok(match self.profiling_strategy {
            ProfilingStrategy::PerfMap => profiling_agent::new_perfmap()?,
            ProfilingStrategy::JitDump => profiling_agent::new_jitdump()?,
            ProfilingStrategy::VTune => profiling_agent::new_vtune()?,
            ProfilingStrategy::None => profiling_agent::new_null(),
        })
    }

    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub(crate) fn build_compiler(
        mut self,
        tunables: &Tunables,
    ) -> Result<(Self, Box<dyn wasmtime_environ::Compiler>)> {
        let target = self.compiler_config.target.clone();

        let mut compiler = match self.compiler_config.strategy {
            #[cfg(feature = "cranelift")]
            Strategy::Auto => wasmtime_cranelift::builder(target)?,
            #[cfg(all(feature = "winch", not(feature = "cranelift")))]
            Strategy::Auto => wasmtime_winch::builder(target)?,
            #[cfg(feature = "cranelift")]
            Strategy::Cranelift => wasmtime_cranelift::builder(target)?,
            #[cfg(not(feature = "cranelift"))]
            Strategy::Cranelift => bail!("cranelift support not compiled in"),
            #[cfg(feature = "winch")]
            Strategy::Winch => wasmtime_winch::builder(target)?,
            #[cfg(not(feature = "winch"))]
            Strategy::Winch => bail!("winch support not compiled in"),
        };

        if let Some(path) = &self.compiler_config.clif_dir {
            compiler.clif_dir(path)?;
        }

        // If probestack is enabled for a target, Wasmtime will always use the
        // inline strategy which doesn't require us to define a `__probestack`
        // function or similar.
        self.compiler_config
            .settings
            .insert("probestack_strategy".into(), "inline".into());

        let host = target_lexicon::Triple::host();
        let target = self
            .compiler_config
            .target
            .as_ref()
            .unwrap_or(&host)
            .clone();

        // On supported targets, we enable stack probing by default.
        // This is required on Windows because of the way Windows
        // commits its stacks, but it's also a good idea on other
        // platforms to ensure guard pages are hit for large frame
        // sizes.
        if probestack_supported(target.architecture) {
            self.compiler_config
                .flags
                .insert("enable_probestack".into());
        }

        if self.features.contains(WasmFeatures::TAIL_CALL) {
            ensure!(
                target.architecture != Architecture::S390x,
                "Tail calls are not supported on s390x yet: \
                 https://github.com/bytecodealliance/wasmtime/issues/6530"
            );
        }

        if let Some(unwind_requested) = self.native_unwind_info {
            if !self
                .compiler_config
                .ensure_setting_unset_or_given("unwind_info", &unwind_requested.to_string())
            {
                bail!("incompatible settings requested for Cranelift and Wasmtime `unwind-info` settings");
            }
        }

        if target.operating_system == target_lexicon::OperatingSystem::Windows {
            if !self
                .compiler_config
                .ensure_setting_unset_or_given("unwind_info", "true")
            {
                bail!("`native_unwind_info` cannot be disabled on Windows");
            }
        }

        // We require frame pointers for correct stack walking, which is safety
        // critical in the presence of reference types, and otherwise it is just
        // really bad developer experience to get wrong.
        self.compiler_config
            .settings
            .insert("preserve_frame_pointers".into(), "true".into());

        // check for incompatible compiler options and set required values
        if self.features.contains(WasmFeatures::REFERENCE_TYPES) {
            if !self
                .compiler_config
                .ensure_setting_unset_or_given("enable_safepoints", "true")
            {
                bail!("compiler option 'enable_safepoints' must be enabled when 'reference types' is enabled");
            }
        }

        if self.features.contains(WasmFeatures::RELAXED_SIMD)
            && !self.features.contains(WasmFeatures::SIMD)
        {
            bail!("cannot disable the simd proposal but enable the relaxed simd proposal");
        }

        // Apply compiler settings and flags
        for (k, v) in self.compiler_config.settings.iter() {
            compiler.set(k, v)?;
        }
        for flag in self.compiler_config.flags.iter() {
            compiler.enable(flag)?;
        }

        #[cfg(feature = "incremental-cache")]
        if let Some(cache_store) = &self.compiler_config.cache_store {
            compiler.enable_incremental_compilation(cache_store.clone())?;
        }

        compiler.set_tunables(tunables.clone())?;
        compiler.wmemcheck(self.compiler_config.wmemcheck);

        Ok((self, compiler.build()?))
    }

    /// Internal setting for whether adapter modules for components will have
    /// extra WebAssembly instructions inserted performing more debug checks
    /// then are necessary.
    #[cfg(feature = "component-model")]
    pub fn debug_adapter_modules(&mut self, debug: bool) -> &mut Self {
        self.tunables.debug_adapter_modules = Some(debug);
        self
    }

    /// Enables clif output when compiling a WebAssembly module.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn emit_clif(&mut self, path: &Path) -> &mut Self {
        self.compiler_config.clif_dir = Some(path.to_path_buf());
        self
    }

    /// Configures whether, when on macOS, Mach ports are used for exception
    /// handling instead of traditional Unix-based signal handling.
    ///
    /// WebAssembly traps in Wasmtime are implemented with native faults, for
    /// example a `SIGSEGV` will occur when a WebAssembly guest accesses
    /// out-of-bounds memory. Handling this can be configured to either use Unix
    /// signals or Mach ports on macOS. By default Mach ports are used.
    ///
    /// Mach ports enable Wasmtime to work by default with foreign
    /// error-handling systems such as breakpad which also use Mach ports to
    /// handle signals. In this situation Wasmtime will continue to handle guest
    /// faults gracefully while any non-guest faults will get forwarded to
    /// process-level handlers such as breakpad. Some more background on this
    /// can be found in #2456.
    ///
    /// A downside of using mach ports, however, is that they don't interact
    /// well with `fork()`. Forking a Wasmtime process on macOS will produce a
    /// child process that cannot successfully run WebAssembly. In this
    /// situation traditional Unix signal handling should be used as that's
    /// inherited and works across forks.
    ///
    /// If your embedding wants to use a custom error handler which leverages
    /// Mach ports and you additionally wish to `fork()` the process and use
    /// Wasmtime in the child process that's not currently possible. Please
    /// reach out to us if you're in this bucket!
    ///
    /// This option defaults to `true`, using Mach ports by default.
    pub fn macos_use_mach_ports(&mut self, mach_ports: bool) -> &mut Self {
        self.macos_use_mach_ports = mach_ports;
        self
    }
}

/// If building without the runtime feature we can't determine the page size of
/// the platform where the execution will happen so just keep the original
/// values.
#[cfg(not(feature = "runtime"))]
fn round_up_to_pages(val: u64) -> u64 {
    val
}

#[cfg(feature = "runtime")]
fn round_up_to_pages(val: u64) -> u64 {
    let page_size = wasmtime_runtime::page_size() as u64;
    debug_assert!(page_size.is_power_of_two());
    val.checked_add(page_size - 1)
        .map(|val| val & !(page_size - 1))
        .unwrap_or(u64::MAX / page_size + 1)
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("Config");
        f.field("debug_info", &self.tunables.generate_native_debuginfo)
            .field(
                "wasm_threads",
                &self.features.contains(WasmFeatures::THREADS),
            )
            .field(
                "wasm_reference_types",
                &self.features.contains(WasmFeatures::REFERENCE_TYPES),
            )
            .field(
                "wasm_function_references",
                &self.features.contains(WasmFeatures::FUNCTION_REFERENCES),
            )
            .field("wasm_gc", &self.features.contains(WasmFeatures::GC))
            .field(
                "wasm_bulk_memory",
                &self.features.contains(WasmFeatures::BULK_MEMORY),
            )
            .field("wasm_simd", &self.features.contains(WasmFeatures::SIMD))
            .field(
                "wasm_relaxed_simd",
                &self.features.contains(WasmFeatures::RELAXED_SIMD),
            )
            .field(
                "wasm_multi_value",
                &self.features.contains(WasmFeatures::MULTI_VALUE),
            )
            .field("parallel_compilation", &self.parallel_compilation);
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        {
            f.field("compiler_config", &self.compiler_config);
        }

        if let Some(enable) = self.tunables.parse_wasm_debuginfo {
            f.field("parse_wasm_debuginfo", &enable);
        }
        if let Some(size) = self.tunables.static_memory_bound {
            f.field(
                "static_memory_maximum_size",
                &(u64::from(size) * u64::from(wasmtime_environ::WASM_PAGE_SIZE)),
            );
        }
        if let Some(size) = self.tunables.static_memory_offset_guard_size {
            f.field("static_memory_guard_size", &size);
        }
        if let Some(size) = self.tunables.dynamic_memory_offset_guard_size {
            f.field("dynamic_memory_guard_size", &size);
        }
        if let Some(enable) = self.tunables.guard_before_linear_memory {
            f.field("guard_before_linear_memory", &enable);
        }
        f.finish()
    }
}

/// Possible Compilation strategies for a wasm module.
///
/// This is used as an argument to the [`Config::strategy`] method.
#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum Strategy {
    /// An indicator that the compilation strategy should be automatically
    /// selected.
    ///
    /// This is generally what you want for most projects and indicates that the
    /// `wasmtime` crate itself should make the decision about what the best
    /// code generator for a wasm module is.
    ///
    /// Currently this always defaults to Cranelift, but the default value may
    /// change over time.
    Auto,

    /// Currently the default backend, Cranelift aims to be a reasonably fast
    /// code generator which generates high quality machine code.
    Cranelift,

    /// A baseline compiler for WebAssembly, currently under active development and not ready for
    /// production applications.
    Winch,
}

/// Possible optimization levels for the Cranelift codegen backend.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum OptLevel {
    /// No optimizations performed, minimizes compilation time by disabling most
    /// optimizations.
    None,
    /// Generates the fastest possible code, but may take longer.
    Speed,
    /// Similar to `speed`, but also performs transformations aimed at reducing
    /// code size.
    SpeedAndSize,
}

/// Select which profiling technique to support.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProfilingStrategy {
    /// No profiler support.
    None,

    /// Collect function name information as the "perf map" file format, used with `perf` on Linux.
    PerfMap,

    /// Collect profiling info for "jitdump" file format, used with `perf` on
    /// Linux.
    JitDump,

    /// Collect profiling info using the "ittapi", used with `VTune` on Linux.
    VTune,
}

/// Select how wasm backtrace detailed information is handled.
#[derive(Debug, Clone, Copy)]
pub enum WasmBacktraceDetails {
    /// Support is unconditionally enabled and wasmtime will parse and read
    /// debug information.
    Enable,

    /// Support is disabled, and wasmtime will not parse debug information for
    /// backtrace details.
    Disable,

    /// Support for backtrace details is conditional on the
    /// `WASMTIME_BACKTRACE_DETAILS` environment variable.
    Environment,
}

/// Configuration options used with [`InstanceAllocationStrategy::Pooling`] to
/// change the behavior of the pooling instance allocator.
///
/// This structure has a builder-style API in the same manner as [`Config`] and
/// is configured with [`Config::allocation_strategy`].
///
/// Note that usage of the pooling allocator does not affect compiled
/// WebAssembly code. Compiled `*.cwasm` files, for example, are usable both
/// with and without the pooling allocator.
///
/// ## Advantages of Pooled Allocation
///
/// The main benefit of the pooling allocator is to make WebAssembly
/// instantiation both faster and more scalable in terms of parallelism.
/// Allocation is faster because virtual memory is already configured and ready
/// to go within the pool, there's no need to [`mmap`] (for example on Unix) a
/// new region and configure it with guard pages. By avoiding [`mmap`] this
/// avoids whole-process virtual memory locks which can improve scalability and
/// performance through avoiding this.
///
/// Additionally with pooled allocation it's possible to create "affine slots"
/// to a particular WebAssembly module or component over time. For example if
/// the same module is multiple times over time the pooling allocator will, by
/// default, attempt to reuse the same slot. This mean that the slot has been
/// pre-configured and can retain virtual memory mappings for a copy-on-write
/// image, for example (see [`Config::memory_init_cow`] for more information.
/// This means that in a steady state instance deallocation is a single
/// [`madvise`] to reset linear memory to its original contents followed by a
/// single (optional) [`mprotect`] during the next instantiation to shrink
/// memory back to its original size. Compared to non-pooled allocation this
/// avoids the need to [`mmap`] a new region of memory, [`munmap`] it, and
/// [`mprotect`] regions too.
///
/// Another benefit of pooled allocation is that it's possible to configure
/// things such that no virtual memory management is required at all in a steady
/// state. For example a pooling allocator can be configured with
/// [`Config::memory_init_cow`] disabledd, dynamic bounds checks enabled
/// through
/// [`Config::static_memory_maximum_size(0)`](Config::static_memory_maximum_size),
/// and sufficient space through
/// [`PoolingAllocationConfig::table_keep_resident`] /
/// [`PoolingAllocationConfig::linear_memory_keep_resident`]. With all these
/// options in place no virtual memory tricks are used at all and everything is
/// manually managed by Wasmtime (for example resetting memory is a
/// `memset(0)`). This is not as fast in a single-threaded scenario but can
/// provide benefits in high-parallelism situations as no virtual memory locks
/// or IPIs need happen.
///
/// ## Disadvantages of Pooled Allocation
///
/// Despite the above advantages to instantiation performance the pooling
/// allocator is not enabled by default in Wasmtime. One reason is that the
/// performance advantages are not necessarily portable, for example while the
/// pooling allocator works on Windows it has not been tuned for performance on
/// Windows in the same way it has on Linux.
///
/// Additionally the main cost of the pooling allocator is that it requires a
/// very large reservation of virtual memory (on the order of most of the
/// addressable virtual address space). WebAssembly 32-bit linear memories in
/// Wasmtime are, by default 4G address space reservations with a 2G guard
/// region both before and after the linear memory. Memories in the pooling
/// allocator are contiguous which means that we only need a guard after linear
/// memory because the previous linear memory's slot post-guard is our own
/// pre-guard. This means that, by default, the pooling allocator uses 6G of
/// virtual memory per WebAssembly linear memory slot. 6G of virtual memory is
/// 32.5 bits of a 64-bit address. Many 64-bit systems can only actually use
/// 48-bit addresses by default (although this can be extended on architectures
/// nowadays too), and of those 48 bits one of them is reserved to indicate
/// kernel-vs-userspace. This leaves 47-32.5=14.5 bits left, meaning you can
/// only have at most 64k slots of linear memories on many systems by default.
/// This is a relatively small number and shows how the pooling allocator can
/// quickly exhaust all of virtual memory.
///
/// Another disadvantage of the pooling allocator is that it may keep memory
/// alive when nothing is using it. A previously used slot for an instance might
/// have paged-in memory that will not get paged out until the
/// [`Engine`](crate::Engine) owning the pooling allocator is dropped. While
/// suitable for some applications this behavior may not be suitable for all
/// applications.
///
/// Finally the last disadvantage of the pooling allocator is that the
/// configuration values for the maximum number of instances, memories, tables,
/// etc, must all be fixed up-front. There's not always a clear answer as to
/// what these values should be so not all applications may be able to work
/// with this constraint.
///
/// [`madvise`]: https://man7.org/linux/man-pages/man2/madvise.2.html
/// [`mprotect`]: https://man7.org/linux/man-pages/man2/mprotect.2.html
/// [`mmap`]: https://man7.org/linux/man-pages/man2/mmap.2.html
/// [`munmap`]: https://man7.org/linux/man-pages/man2/munmap.2.html
#[cfg(feature = "pooling-allocator")]
#[derive(Debug, Clone, Default)]
pub struct PoolingAllocationConfig {
    config: wasmtime_runtime::PoolingInstanceAllocatorConfig,
}

#[cfg(feature = "pooling-allocator")]
impl PoolingAllocationConfig {
    /// Configures the maximum number of "unused warm slots" to retain in the
    /// pooling allocator.
    ///
    /// The pooling allocator operates over slots to allocate from, and each
    /// slot is considered "cold" if it's never been used before or "warm" if
    /// it's been used by some module in the past. Slots in the pooling
    /// allocator additionally track an "affinity" flag to a particular core
    /// wasm module. When a module is instantiated into a slot then the slot is
    /// considered affine to that module, even after the instance has been
    /// deallocated.
    ///
    /// When a new instance is created then a slot must be chosen, and the
    /// current algorithm for selecting a slot is:
    ///
    /// * If there are slots that are affine to the module being instantiated,
    ///   then the most recently used slot is selected to be allocated from.
    ///   This is done to improve reuse of resources such as memory mappings and
    ///   additionally try to benefit from temporal locality for things like
    ///   caches.
    ///
    /// * Otherwise if there are more than N affine slots to other modules, then
    ///   one of those affine slots is chosen to be allocated. The slot chosen
    ///   is picked on a least-recently-used basis.
    ///
    /// * Finally, if there are less than N affine slots to other modules, then
    ///   the non-affine slots are allocated from.
    ///
    /// This setting, `max_unused_warm_slots`, is the value for N in the above
    /// algorithm. The purpose of this setting is to have a knob over the RSS
    /// impact of "unused slots" for a long-running wasm server.
    ///
    /// If this setting is set to 0, for example, then affine slots are
    /// aggressively reused on a least-recently-used basis. A "cold" slot is
    /// only used if there are no affine slots available to allocate from. This
    /// means that the set of slots used over the lifetime of a program is the
    /// same as the maximum concurrent number of wasm instances.
    ///
    /// If this setting is set to infinity, however, then cold slots are
    /// prioritized to be allocated from. This means that the set of slots used
    /// over the lifetime of a program will approach
    /// [`PoolingAllocationConfig::total_memories`], or the maximum number of
    /// slots in the pooling allocator.
    ///
    /// Wasmtime does not aggressively decommit all resources associated with a
    /// slot when the slot is not in use. For example the
    /// [`PoolingAllocationConfig::linear_memory_keep_resident`] option can be
    /// used to keep memory associated with a slot, even when it's not in use.
    /// This means that the total set of used slots in the pooling instance
    /// allocator can impact the overall RSS usage of a program.
    ///
    /// The default value for this option is `100`.
    pub fn max_unused_warm_slots(&mut self, max: u32) -> &mut Self {
        self.config.max_unused_warm_slots = max;
        self
    }

    /// Configures whether or not stacks used for async futures are reset to
    /// zero after usage.
    ///
    /// When the [`async_support`](Config::async_support) method is enabled for
    /// Wasmtime and the [`call_async`] variant
    /// of calling WebAssembly is used then Wasmtime will create a separate
    /// runtime execution stack for each future produced by [`call_async`].
    /// During the deallocation process Wasmtime won't by default reset the
    /// contents of the stack back to zero.
    ///
    /// When this option is enabled it can be seen as a defense-in-depth
    /// mechanism to reset a stack back to zero. This is not required for
    /// correctness and can be a costly operation in highly concurrent
    /// environments due to modifications of the virtual address space requiring
    /// process-wide synchronization.
    ///
    /// This option defaults to `false`.
    ///
    /// [`call_async`]: crate::TypedFunc::call_async
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn async_stack_zeroing(&mut self, enable: bool) -> &mut Self {
        self.config.async_stack_zeroing = enable;
        self
    }

    /// How much memory, in bytes, to keep resident for async stacks allocated
    /// with the pooling allocator.
    ///
    /// When [`PoolingAllocationConfig::async_stack_zeroing`] is enabled then
    /// Wasmtime will reset the contents of async stacks back to zero upon
    /// deallocation. This option can be used to perform the zeroing operation
    /// with `memset` up to a certain threshold of bytes instead of using system
    /// calls to reset the stack to zero.
    ///
    /// Note that when using this option the memory with async stacks will
    /// never be decommitted.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn async_stack_keep_resident(&mut self, size: usize) -> &mut Self {
        let size = round_up_to_pages(size as u64) as usize;
        self.config.async_stack_keep_resident = size;
        self
    }

    /// How much memory, in bytes, to keep resident for each linear memory
    /// after deallocation.
    ///
    /// This option is only applicable on Linux and has no effect on other
    /// platforms.
    ///
    /// By default Wasmtime will use `madvise` to reset the entire contents of
    /// linear memory back to zero when a linear memory is deallocated. This
    /// option can be used to use `memset` instead to set memory back to zero
    /// which can, in some configurations, reduce the number of page faults
    /// taken when a slot is reused.
    pub fn linear_memory_keep_resident(&mut self, size: usize) -> &mut Self {
        let size = round_up_to_pages(size as u64) as usize;
        self.config.linear_memory_keep_resident = size;
        self
    }

    /// How much memory, in bytes, to keep resident for each table after
    /// deallocation.
    ///
    /// This option is only applicable on Linux and has no effect on other
    /// platforms.
    ///
    /// This option is the same as
    /// [`PoolingAllocationConfig::linear_memory_keep_resident`] except that it
    /// is applicable to tables instead.
    pub fn table_keep_resident(&mut self, size: usize) -> &mut Self {
        let size = round_up_to_pages(size as u64) as usize;
        self.config.table_keep_resident = size;
        self
    }

    /// The maximum number of concurrent component instances supported (default
    /// is `1000`).
    ///
    /// This provides an upper-bound on the total size of component
    /// metadata-related allocations, along with
    /// [`PoolingAllocationConfig::max_component_instance_size`]. The upper bound is
    ///
    /// ```text
    /// total_component_instances * max_component_instance_size
    /// ```
    ///
    /// where `max_component_instance_size` is rounded up to the size and alignment
    /// of the internal representation of the metadata.
    pub fn total_component_instances(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_component_instances = count;
        self
    }

    /// The maximum size, in bytes, allocated for a component instance's
    /// `VMComponentContext` metadata.
    ///
    /// The [`wasmtime::component::Instance`][crate::component::Instance] type
    /// has a static size but its internal `VMComponentContext` is dynamically
    /// sized depending on the component being instantiated. This size limit
    /// loosely correlates to the size of the component, taking into account
    /// factors such as:
    ///
    /// * number of lifted and lowered functions,
    /// * number of memories
    /// * number of inner instances
    /// * number of resources
    ///
    /// If the allocated size per instance is too small then instantiation of a
    /// module will fail at runtime with an error indicating how many bytes were
    /// needed.
    ///
    /// The default value for this is 1MiB.
    ///
    /// This provides an upper-bound on the total size of component
    /// metadata-related allocations, along with
    /// [`PoolingAllocationConfig::total_component_instances`]. The upper bound is
    ///
    /// ```text
    /// total_component_instances * max_component_instance_size
    /// ```
    ///
    /// where `max_component_instance_size` is rounded up to the size and alignment
    /// of the internal representation of the metadata.
    pub fn max_component_instance_size(&mut self, size: usize) -> &mut Self {
        self.config.limits.component_instance_size = size;
        self
    }

    /// The maximum number of core instances a single component may contain
    /// (default is `20`).
    ///
    /// This method (along with
    /// [`PoolingAllocationConfig::max_memories_per_component`],
    /// [`PoolingAllocationConfig::max_tables_per_component`], and
    /// [`PoolingAllocationConfig::max_component_instance_size`]) allows you to cap
    /// the amount of resources a single component allocation consumes.
    ///
    /// If a component will instantiate more core instances than `count`, then
    /// the component will fail to instantiate.
    pub fn max_core_instances_per_component(&mut self, count: u32) -> &mut Self {
        self.config.limits.max_core_instances_per_component = count;
        self
    }

    /// The maximum number of Wasm linear memories that a single component may
    /// transitively contain (default is `20`).
    ///
    /// This method (along with
    /// [`PoolingAllocationConfig::max_core_instances_per_component`],
    /// [`PoolingAllocationConfig::max_tables_per_component`], and
    /// [`PoolingAllocationConfig::max_component_instance_size`]) allows you to cap
    /// the amount of resources a single component allocation consumes.
    ///
    /// If a component transitively contains more linear memories than `count`,
    /// then the component will fail to instantiate.
    pub fn max_memories_per_component(&mut self, count: u32) -> &mut Self {
        self.config.limits.max_memories_per_component = count;
        self
    }

    /// The maximum number of tables that a single component may transitively
    /// contain (default is `20`).
    ///
    /// This method (along with
    /// [`PoolingAllocationConfig::max_core_instances_per_component`],
    /// [`PoolingAllocationConfig::max_memories_per_component`],
    /// [`PoolingAllocationConfig::max_component_instance_size`]) allows you to cap
    /// the amount of resources a single component allocation consumes.
    ///
    /// If a component will transitively contains more tables than `count`, then
    /// the component will fail to instantiate.
    pub fn max_tables_per_component(&mut self, count: u32) -> &mut Self {
        self.config.limits.max_tables_per_component = count;
        self
    }

    /// The maximum number of concurrent Wasm linear memories supported (default
    /// is `1000`).
    ///
    /// This value has a direct impact on the amount of memory allocated by the pooling
    /// instance allocator.
    ///
    /// The pooling instance allocator allocates a memory pool, where each entry
    /// in the pool contains the reserved address space for each linear memory
    /// supported by an instance.
    ///
    /// The memory pool will reserve a large quantity of host process address
    /// space to elide the bounds checks required for correct WebAssembly memory
    /// semantics. Even with 64-bit address spaces, the address space is limited
    /// when dealing with a large number of linear memories.
    ///
    /// For example, on Linux x86_64, the userland address space limit is 128
    /// TiB. That might seem like a lot, but each linear memory will *reserve* 6
    /// GiB of space by default.
    pub fn total_memories(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_memories = count;
        self
    }

    /// The maximum number of concurrent tables supported (default is `1000`).
    ///
    /// This value has a direct impact on the amount of memory allocated by the
    /// pooling instance allocator.
    ///
    /// The pooling instance allocator allocates a table pool, where each entry
    /// in the pool contains the space needed for each WebAssembly table
    /// supported by an instance (see `table_elements` to control the size of
    /// each table).
    pub fn total_tables(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_tables = count;
        self
    }

    /// The maximum number of execution stacks allowed for asynchronous
    /// execution, when enabled (default is `1000`).
    ///
    /// This value has a direct impact on the amount of memory allocated by the
    /// pooling instance allocator.
    #[cfg(feature = "async")]
    pub fn total_stacks(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_stacks = count;
        self
    }

    /// The maximum number of concurrent core instances supported (default is
    /// `1000`).
    ///
    /// This provides an upper-bound on the total size of core instance
    /// metadata-related allocations, along with
    /// [`PoolingAllocationConfig::max_core_instance_size`]. The upper bound is
    ///
    /// ```text
    /// total_core_instances * max_core_instance_size
    /// ```
    ///
    /// where `max_core_instance_size` is rounded up to the size and alignment of
    /// the internal representation of the metadata.
    pub fn total_core_instances(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_core_instances = count;
        self
    }

    /// The maximum size, in bytes, allocated for a core instance's `VMContext`
    /// metadata.
    ///
    /// The [`Instance`][crate::Instance] type has a static size but its
    /// `VMContext` metadata is dynamically sized depending on the module being
    /// instantiated. This size limit loosely correlates to the size of the Wasm
    /// module, taking into account factors such as:
    ///
    /// * number of functions
    /// * number of globals
    /// * number of memories
    /// * number of tables
    /// * number of function types
    ///
    /// If the allocated size per instance is too small then instantiation of a
    /// module will fail at runtime with an error indicating how many bytes were
    /// needed.
    ///
    /// The default value for this is 1MiB.
    ///
    /// This provides an upper-bound on the total size of core instance
    /// metadata-related allocations, along with
    /// [`PoolingAllocationConfig::total_core_instances`]. The upper bound is
    ///
    /// ```text
    /// total_core_instances * max_core_instance_size
    /// ```
    ///
    /// where `max_core_instance_size` is rounded up to the size and alignment of
    /// the internal representation of the metadata.
    pub fn max_core_instance_size(&mut self, size: usize) -> &mut Self {
        self.config.limits.core_instance_size = size;
        self
    }

    /// The maximum number of defined tables for a core module (default is `1`).
    ///
    /// This value controls the capacity of the `VMTableDefinition` table in
    /// each instance's `VMContext` structure.
    ///
    /// The allocated size of the table will be `tables *
    /// sizeof(VMTableDefinition)` for each instance regardless of how many
    /// tables are defined by an instance's module.
    pub fn max_tables_per_module(&mut self, tables: u32) -> &mut Self {
        self.config.limits.max_tables_per_module = tables;
        self
    }

    /// The maximum table elements for any table defined in a module (default is
    /// `10000`).
    ///
    /// If a table's minimum element limit is greater than this value, the
    /// module will fail to instantiate.
    ///
    /// If a table's maximum element limit is unbounded or greater than this
    /// value, the maximum will be `table_elements` for the purpose of any
    /// `table.grow` instruction.
    ///
    /// This value is used to reserve the maximum space for each supported
    /// table; table elements are pointer-sized in the Wasmtime runtime.
    /// Therefore, the space reserved for each instance is `tables *
    /// table_elements * sizeof::<*const ()>`.
    pub fn table_elements(&mut self, elements: u32) -> &mut Self {
        self.config.limits.table_elements = elements;
        self
    }

    /// The maximum number of defined linear memories for a module (default is
    /// `1`).
    ///
    /// This value controls the capacity of the `VMMemoryDefinition` table in
    /// each core instance's `VMContext` structure.
    ///
    /// The allocated size of the table will be `memories *
    /// sizeof(VMMemoryDefinition)` for each core instance regardless of how
    /// many memories are defined by the core instance's module.
    pub fn max_memories_per_module(&mut self, memories: u32) -> &mut Self {
        self.config.limits.max_memories_per_module = memories;
        self
    }

    /// The maximum number of Wasm pages for any linear memory defined in a
    /// module (default is `160`).
    ///
    /// The default of `160` means at most 10 MiB of host memory may be
    /// committed for each instance.
    ///
    /// If a memory's minimum page limit is greater than this value, the module
    /// will fail to instantiate.
    ///
    /// If a memory's maximum page limit is unbounded or greater than this
    /// value, the maximum will be `memory_pages` for the purpose of any
    /// `memory.grow` instruction.
    ///
    /// This value is used to control the maximum accessible space for each
    /// linear memory of a core instance.
    ///
    /// The reservation size of each linear memory is controlled by the
    /// `static_memory_maximum_size` setting and this value cannot exceed the
    /// configured static memory maximum size.
    pub fn memory_pages(&mut self, pages: u64) -> &mut Self {
        self.config.limits.memory_pages = pages;
        self
    }

    /// Configures whether memory protection keys (MPK) should be used for more
    /// efficient layout of pool-allocated memories.
    ///
    /// When using the pooling allocator (see [`Config::allocation_strategy`],
    /// [`InstanceAllocationStrategy::Pooling`]), memory protection keys can
    /// reduce the total amount of allocated virtual memory by eliminating guard
    /// regions between WebAssembly memories in the pool. It does so by
    /// "coloring" memory regions with different memory keys and setting which
    /// regions are accessible each time executions switches from host to guest
    /// (or vice versa).
    ///
    /// MPK is only available on Linux (called `pku` there) and recent x86
    /// systems; we check for MPK support at runtime by examining the `CPUID`
    /// register. This configuration setting can be in three states:
    ///
    /// - `auto`: if MPK support is available the guard regions are removed; if
    ///   not, the guard regions remain
    /// - `enable`: use MPK to eliminate guard regions; fail if MPK is not
    ///   supported
    /// - `disable`: never use MPK
    ///
    /// By default this value is `disabled`, but may become `auto` in future
    /// releases.
    ///
    /// __WARNING__: this configuration options is still experimental--use at
    /// your own risk! MPK uses kernel and CPU features to protect memory
    /// regions; you may observe segmentation faults if anything is
    /// misconfigured.
    pub fn memory_protection_keys(&mut self, enable: MpkEnabled) -> &mut Self {
        self.config.memory_protection_keys = enable;
        self
    }

    /// Sets an upper limit on how many memory protection keys (MPK) Wasmtime
    /// will use.
    ///
    /// This setting is only applicable when
    /// [`PoolingAllocationConfig::memory_protection_keys`] is set to `enable`
    /// or `auto`. Configuring this above the HW and OS limits (typically 15)
    /// has no effect.
    ///
    /// If multiple Wasmtime engines are used in the same process, note that all
    /// engines will share the same set of allocated keys; this setting will
    /// limit how many keys are allocated initially and thus available to all
    /// other engines.
    pub fn max_memory_protection_keys(&mut self, max: usize) -> &mut Self {
        self.config.max_memory_protection_keys = max;
        self
    }

    /// Check if memory protection keys (MPK) are available on the current host.
    ///
    /// This is a convenience method for determining MPK availability using the
    /// same method that [`MpkEnabled::Auto`] does. See
    /// [`PoolingAllocationConfig::memory_protection_keys`] for more
    /// information.
    pub fn are_memory_protection_keys_available() -> bool {
        mpk::is_supported()
    }

    /// The maximum number of concurrent GC heaps supported (default is `1000`).
    ///
    /// This value has a direct impact on the amount of memory allocated by the
    /// pooling instance allocator.
    ///
    /// The pooling instance allocator allocates a GC heap pool, where each
    /// entry in the pool contains the space needed for each GC heap used by a
    /// store.
    #[cfg(feature = "gc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gc")))]
    pub fn total_gc_heaps(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_gc_heaps = count;
        self
    }
}

pub(crate) fn probestack_supported(arch: Architecture) -> bool {
    matches!(
        arch,
        Architecture::X86_64 | Architecture::Aarch64(_) | Architecture::Riscv64(_)
    )
}
