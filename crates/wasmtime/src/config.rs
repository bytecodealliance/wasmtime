use crate::prelude::*;
use alloc::sync::Arc;
use bitflags::Flags;
use core::fmt;
use core::num::NonZeroUsize;
use core::str::FromStr;
#[cfg(any(feature = "cache", feature = "cranelift", feature = "winch"))]
use std::path::Path;
pub use wasmparser::WasmFeatures;
use wasmtime_environ::{ConfigTunables, TripleExt, Tunables};

#[cfg(feature = "runtime")]
use crate::memory::MemoryCreator;
#[cfg(feature = "runtime")]
use crate::profiling_agent::{self, ProfilingAgent};
#[cfg(feature = "runtime")]
use crate::runtime::vm::{
    GcRuntime, InstanceAllocator, OnDemandInstanceAllocator, RuntimeMemoryCreator,
};
#[cfg(feature = "runtime")]
use crate::trampoline::MemoryCreatorProxy;

#[cfg(feature = "async")]
use crate::stack::{StackCreator, StackCreatorProxy};
#[cfg(feature = "async")]
use wasmtime_fiber::RuntimeFiberStackCreator;

#[cfg(feature = "runtime")]
pub use crate::runtime::code_memory::CustomCodeMemory;
#[cfg(feature = "cache")]
pub use wasmtime_cache::{Cache, CacheConfig};
#[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
pub use wasmtime_environ::CacheStore;

pub(crate) const DEFAULT_WASM_BACKTRACE_MAX_FRAMES: NonZeroUsize = NonZeroUsize::new(20).unwrap();

/// Represents the module instance allocation strategy to use.
#[derive(Clone)]
#[non_exhaustive]
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

#[cfg(feature = "pooling-allocator")]
impl From<PoolingAllocationConfig> for InstanceAllocationStrategy {
    fn from(cfg: PoolingAllocationConfig) -> InstanceAllocationStrategy {
        InstanceAllocationStrategy::Pooling(cfg)
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

impl core::hash::Hash for ModuleVersionStrategy {
    fn hash<H: core::hash::Hasher>(&self, hasher: &mut H) {
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
///
/// # Defaults
///
/// The `Default` trait implementation and the return value from
/// [`Config::new()`] are the same and represent the default set of
/// configuration for an engine. The exact set of defaults will differ based on
/// properties such as enabled Cargo features at compile time and the configured
/// target (see [`Config::target`]). Configuration options document their
/// default values and what the conditional value of the default is where
/// applicable.
#[derive(Clone)]
pub struct Config {
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    compiler_config: Option<CompilerConfig>,
    target: Option<target_lexicon::Triple>,
    #[cfg(feature = "gc")]
    collector: Collector,
    profiling_strategy: ProfilingStrategy,
    tunables: ConfigTunables,

    #[cfg(feature = "cache")]
    pub(crate) cache: Option<Cache>,
    #[cfg(feature = "runtime")]
    pub(crate) mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>,
    #[cfg(feature = "runtime")]
    pub(crate) custom_code_memory: Option<Arc<dyn CustomCodeMemory>>,
    pub(crate) allocation_strategy: InstanceAllocationStrategy,
    pub(crate) max_wasm_stack: usize,
    /// Explicitly enabled features via `Config::wasm_*` methods. This is a
    /// signal that the embedder specifically wants something turned on
    /// regardless of the defaults that Wasmtime might otherwise have enabled.
    ///
    /// Note that this, and `disabled_features` below, start as the empty set of
    /// features to only track explicit user requests.
    pub(crate) enabled_features: WasmFeatures,
    /// Same as `enabled_features`, but for those that are explicitly disabled.
    pub(crate) disabled_features: WasmFeatures,
    pub(crate) wasm_backtrace_details_env_used: bool,
    pub(crate) wasm_backtrace_max_frames: Option<NonZeroUsize>,
    pub(crate) native_unwind_info: Option<bool>,
    #[cfg(any(feature = "async", feature = "stack-switching"))]
    pub(crate) async_stack_size: usize,
    #[cfg(feature = "async")]
    pub(crate) async_stack_zeroing: bool,
    #[cfg(feature = "async")]
    pub(crate) stack_creator: Option<Arc<dyn RuntimeFiberStackCreator>>,
    pub(crate) module_version: ModuleVersionStrategy,
    pub(crate) parallel_compilation: bool,
    pub(crate) memory_guaranteed_dense_image_size: u64,
    pub(crate) force_memory_init_memfd: bool,
    pub(crate) wmemcheck: bool,
    #[cfg(feature = "coredump")]
    pub(crate) coredump_on_trap: bool,
    pub(crate) macos_use_mach_ports: bool,
    pub(crate) detect_host_feature: Option<fn(&str) -> Option<bool>>,
    pub(crate) x86_float_abi_ok: Option<bool>,
    pub(crate) shared_memory: bool,
}

/// User-provided configuration for the compiler.
#[cfg(any(feature = "cranelift", feature = "winch"))]
#[derive(Debug, Clone)]
struct CompilerConfig {
    strategy: Option<Strategy>,
    settings: crate::hash_map::HashMap<String, String>,
    flags: crate::hash_set::HashSet<String>,
    #[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
    cache_store: Option<Arc<dyn CacheStore>>,
    clif_dir: Option<std::path::PathBuf>,
    wmemcheck: bool,
}

#[cfg(any(feature = "cranelift", feature = "winch"))]
impl CompilerConfig {
    fn new() -> Self {
        Self {
            strategy: Strategy::Auto.not_auto(),
            settings: Default::default(),
            flags: Default::default(),
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
        Self::new()
    }
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        let mut ret = Self {
            tunables: ConfigTunables::default(),
            #[cfg(any(feature = "cranelift", feature = "winch"))]
            compiler_config: Some(CompilerConfig::default()),
            target: None,
            #[cfg(feature = "gc")]
            collector: Collector::default(),
            #[cfg(feature = "cache")]
            cache: None,
            profiling_strategy: ProfilingStrategy::None,
            #[cfg(feature = "runtime")]
            mem_creator: None,
            #[cfg(feature = "runtime")]
            custom_code_memory: None,
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
            wasm_backtrace_details_env_used: false,
            wasm_backtrace_max_frames: Some(DEFAULT_WASM_BACKTRACE_MAX_FRAMES),
            native_unwind_info: None,
            enabled_features: WasmFeatures::empty(),
            disabled_features: WasmFeatures::empty(),
            #[cfg(any(feature = "async", feature = "stack-switching"))]
            async_stack_size: 2 << 20,
            #[cfg(feature = "async")]
            async_stack_zeroing: false,
            #[cfg(feature = "async")]
            stack_creator: None,
            module_version: ModuleVersionStrategy::default(),
            parallel_compilation: !cfg!(miri),
            memory_guaranteed_dense_image_size: 16 << 20,
            force_memory_init_memfd: false,
            wmemcheck: false,
            #[cfg(feature = "coredump")]
            coredump_on_trap: false,
            macos_use_mach_ports: !cfg!(miri),
            #[cfg(feature = "std")]
            detect_host_feature: Some(detect_host_feature),
            #[cfg(not(feature = "std"))]
            detect_host_feature: None,
            x86_float_abi_ok: None,
            shared_memory: false,
        };
        ret.wasm_backtrace_details(WasmBacktraceDetails::Environment);
        ret
    }

    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub(crate) fn has_compiler(&self) -> bool {
        self.compiler_config.is_some()
    }

    #[track_caller]
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    fn compiler_config_mut(&mut self) -> &mut CompilerConfig {
        self.compiler_config.as_mut().expect(
            "cannot configure compiler settings for `Config`s \
             created by `Config::without_compiler`",
        )
    }

    /// Configure whether Wasm compilation is enabled.
    ///
    /// Disabling Wasm compilation will allow you to load and run
    /// [pre-compiled][crate::Engine::precompile_module] Wasm programs, but not
    /// to compile and run new Wasm programs that have not already been
    /// pre-compiled.
    ///
    /// Many compilation-related configuration methods will panic if compilation
    /// has been disabled.
    ///
    /// Note that there are two ways to disable Wasm compilation:
    ///
    /// 1. Statically, by disabling the `"cranelift"` and `"winch"` cargo
    ///    features when building Wasmtime. These builds of Wasmtime will have
    ///    smaller code size, since they do not include any of the code to
    ///    compile Wasm.
    ///
    /// 2. Dynamically, by passing `false` to this method at run-time when
    ///    configuring Wasmtime. The Wasmtime binary will still include the code
    ///    for compiling Wasm, it just won't be executed, so code size is larger
    ///    than with the first approach.
    ///
    /// The static approach is better in most cases, however dynamically calling
    /// `enable_compiler(false)` is useful whenever you create multiple
    /// `Engine`s in the same process, some of which must be able to compile
    /// Wasm and some of which should never do so. Tests are a common example of
    /// such a situation, especially when there are multiple Rust binaries in
    /// the same cargo workspace, and cargo's feature resolution enables the
    /// `"cranelift"` or `"winch"` features across the whole workspace.
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn enable_compiler(&mut self, enable: bool) -> &mut Self {
        match (enable, &self.compiler_config) {
            (true, Some(_)) | (false, None) => {}
            (true, None) => {
                self.compiler_config = Some(CompilerConfig::default());
            }
            (false, Some(_)) => {
                self.compiler_config = None;
            }
        }
        self
    }

    /// Configures the target platform of this [`Config`].
    ///
    /// This method is used to configure the output of compilation in an
    /// [`Engine`](crate::Engine). This can be used, for example, to
    /// cross-compile from one platform to another. By default, the host target
    /// triple is used meaning compiled code is suitable to run on the host.
    ///
    /// Note that the [`Module`](crate::Module) type can only be created if the
    /// target configured here matches the host. Otherwise if a cross-compile is
    /// being performed where the host doesn't match the target then
    /// [`Engine::precompile_module`](crate::Engine::precompile_module) must be
    /// used instead.
    ///
    /// Target-specific flags (such as CPU features) will not be inferred by
    /// default for the target when one is provided here. This means that this
    /// can also be used, for example, with the host architecture to disable all
    /// host-inferred feature flags. Configuring target-specific flags can be
    /// done with [`Config::cranelift_flag_set`] and
    /// [`Config::cranelift_flag_enable`].
    ///
    /// # Errors
    ///
    /// This method will error if the given target triple is not supported.
    pub fn target(&mut self, target: &str) -> Result<&mut Self> {
        self.target =
            Some(target_lexicon::Triple::from_str(target).map_err(|e| crate::format_err!(e))?);

        Ok(self)
    }

    /// Enables the incremental compilation cache in Cranelift, using the provided `CacheStore`
    /// backend for storage.
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
    pub fn enable_incremental_compilation(
        &mut self,
        cache_store: Arc<dyn CacheStore>,
    ) -> Result<&mut Self> {
        self.compiler_config_mut().cache_store = Some(cache_store);
        Ok(self)
    }

    #[doc(hidden)]
    #[deprecated(note = "no longer has any effect")]
    #[cfg(feature = "async")]
    pub fn async_support(&mut self, _enable: bool) -> &mut Self {
        self
    }

    /// Configures whether DWARF debug information will be emitted
    /// during compilation for a native debugger on the Wasmtime
    /// process to consume.
    ///
    /// Note that the `debug-builtins` compile-time Cargo feature must also be
    /// enabled for native debuggers such as GDB or LLDB to be able to debug
    /// guest WebAssembly programs.
    ///
    /// By default this option is `false`.
    /// **Note** Enabling this option is not compatible with the Winch compiler.
    pub fn debug_info(&mut self, enable: bool) -> &mut Self {
        self.tunables.debug_native = Some(enable);
        self
    }

    /// Configures whether compiled guest code will be instrumented to
    /// provide debugging at the Wasm VM level.
    ///
    /// This is required in order to enable a guest-level debugging
    /// API that can precisely examine Wasm VM state and (eventually,
    /// once it is complete) set breakpoints and watchpoints and step
    /// through code.
    ///
    /// Without this enabled, debugging can only be done via a native
    /// debugger operating on the compiled guest code (see
    /// [`Config::debug_info`] and is "best-effort": we may be able to
    /// recover some Wasm locals or operand stack values, but it is
    /// not guaranteed, even when optimizations are disabled.
    ///
    /// When this is enabled, additional instrumentation is inserted
    /// that directly tracks the Wasm VM state at every step. This has
    /// some performance impact, but allows perfect debugging
    /// fidelity.
    ///
    /// Breakpoints, watchpoints, and stepping are not yet supported,
    /// but will be added in a future version of Wasmtime.
    ///
    /// This enables use of the [`crate::DebugFrameCursor`] API which is
    /// provided by [`crate::Caller::debug_frames`] from within a
    /// hostcall context.
    ///
    /// ***Note*** Enabling this option is not compatible with the
    /// Winch compiler.
    #[cfg(feature = "debug")]
    pub fn guest_debug(&mut self, enable: bool) -> &mut Self {
        self.tunables.debug_guest = Some(enable);
        self
    }

    /// Configures whether [`WasmBacktrace`] will be present in the context of
    /// errors returned from Wasmtime.
    ///
    /// This method is deprecated in favor of
    /// [`Config::wasm_backtrace_max_frames`]. Calling `wasm_backtrace(false)`
    /// is equivalent to `wasm_backtrace_max_frames(None)`, and
    /// `wasm_backtrace(true)` will leave `wasm_backtrace_max_frames` unchanged
    /// if the value is `Some` and will otherwise restore the default `Some`
    /// value.
    ///
    /// [`WasmBacktrace`]: crate::WasmBacktrace
    #[deprecated = "use `wasm_backtrace_max_frames` instead"]
    pub fn wasm_backtrace(&mut self, enable: bool) -> &mut Self {
        match (enable, self.wasm_backtrace_max_frames) {
            (false, _) => self.wasm_backtrace_max_frames = None,
            // Wasm backtraces were disabled; enable them with the
            // default maximum number of frames to capture.
            (true, None) => {
                self.wasm_backtrace_max_frames = Some(DEFAULT_WASM_BACKTRACE_MAX_FRAMES)
            }
            // Wasm backtraces are already enabled; keep the existing
            // max-frames configuration.
            (true, Some(_)) => {}
        }
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
    /// that wasm will read `WASMTIME_BACKTRACE_DETAILS` to indicate whether
    /// details should be parsed. Note that the `std` feature of this crate must
    /// be active to read environment variables, otherwise this is disabled by
    /// default.
    pub fn wasm_backtrace_details(&mut self, enable: WasmBacktraceDetails) -> &mut Self {
        self.wasm_backtrace_details_env_used = false;
        self.tunables.parse_wasm_debuginfo = match enable {
            WasmBacktraceDetails::Enable => Some(true),
            WasmBacktraceDetails::Disable => Some(false),
            WasmBacktraceDetails::Environment => {
                #[cfg(feature = "std")]
                {
                    self.wasm_backtrace_details_env_used = true;
                    std::env::var("WASMTIME_BACKTRACE_DETAILS")
                        .map(|s| Some(s == "1"))
                        .unwrap_or(Some(false))
                }
                #[cfg(not(feature = "std"))]
                {
                    Some(false)
                }
            }
        };
        self
    }

    /// Configures the maximum number of WebAssembly frames to collect in
    /// backtraces.
    ///
    /// A backtrace may be collected whenever an error is returned from a host
    /// function call through to WebAssembly or when WebAssembly itself hits a
    /// trap condition, such as an out-of-bounds memory access. This flag
    /// indicates, in these conditions, whether the backtrace is collected or
    /// not and how many frames should be collected.
    ///
    /// Currently wasm backtraces are implemented through frame pointer walking.
    /// This means that collecting a backtrace is expected to be a fast and
    /// relatively cheap operation. Additionally backtrace collection is
    /// suitable in concurrent environments since one thread capturing a
    /// backtrace won't block other threads.
    ///
    /// Collected backtraces are attached via
    /// [`Error::context`](crate::Error::context) to errors returned from host
    /// functions. The [`WasmBacktrace`] type can be acquired via
    /// [`Error::downcast_ref`](crate::Error::downcast_ref) to inspect the
    /// backtrace. When this option is set to `None` then this context is never
    /// applied to errors coming out of wasm.
    ///
    /// The default value is 20.
    pub fn wasm_backtrace_max_frames(&mut self, limit: Option<NonZeroUsize>) -> &mut Self {
        self.wasm_backtrace_max_frames = limit;
        self
    }

    /// Configures whether to generate native unwind information
    /// (e.g. `.eh_frame` on Linux).
    ///
    /// This configuration option only exists to help third-party stack
    /// capturing mechanisms, such as the system's unwinder or the `backtrace`
    /// crate, determine how to unwind through Wasm frames. It does not affect
    /// whether Wasmtime can capture Wasm backtraces or not. The presence of
    /// [`WasmBacktrace`] is controlled by the
    /// [`Config::wasm_backtrace_max_frames`] option.
    ///
    /// Native unwind information is included:
    /// - When targeting Windows, since the Windows ABI requires it.
    /// - By default.
    ///
    /// Note that systems loading many modules may wish to disable this
    /// configuration option instead of leaving it on-by-default. Some platforms
    /// exhibit quadratic behavior when registering/unregistering unwinding
    /// information which can greatly slow down the module loading/unloading
    /// process.
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
    /// **Note** Enabling this option is not compatible with the Winch compiler.
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
    /// ## Interaction with blocking host calls
    ///
    /// Epochs (and fuel) do not assist in handling WebAssembly code blocked in
    /// a call to the host. For example if the WebAssembly function calls
    /// `wasi:io/poll.poll` to sleep epochs will not assist in waking this up or
    /// timing it out. Epochs intentionally only affect running WebAssembly code
    /// itself and it's left to the embedder to determine how best to wake up
    /// indefinitely blocking code in the host.
    ///
    /// The typical solution for this, however, is to use the `async` variant of
    /// WASI host functions. This models computation as a Rust `Future` which
    /// means that when blocking happens the future is only suspended and
    /// control yields back to the main event loop. This gives the embedder the
    /// opportunity to use `tokio::time::timeout` for example on a wasm
    /// computation and have the desired effect of cancelling a blocking
    /// operation when a timeout expires.
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
    /// **Note** Enabling this option is not compatible with the Winch compiler.
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
    /// back into wasm, then the execution will trap immediately because of
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
    #[cfg(any(feature = "async", feature = "stack-switching"))]
    pub fn async_stack_size(&mut self, size: usize) -> &mut Self {
        self.async_stack_size = size;
        self
    }

    /// Configures whether or not stacks used for async futures are zeroed
    /// before (re)use.
    ///
    /// When the [`call_async`] variant of calling WebAssembly is used
    /// then Wasmtime will create a separate runtime execution stack for each
    /// future produced by [`call_async`]. By default upon allocation, depending
    /// on the platform, these stacks might be filled with uninitialized
    /// memory. This is safe and correct because, modulo bugs in Wasmtime,
    /// compiled Wasm code will never read from a stack slot before it
    /// initializes the stack slot.
    ///
    /// However, as a defense-in-depth mechanism, you may configure Wasmtime to
    /// ensure that these stacks are zeroed before they are used. Notably, if
    /// you are using the pooling allocator, stacks can be pooled and reused
    /// across different Wasm guests; ensuring that stacks are zeroed can
    /// prevent data leakage between Wasm guests even in the face of potential
    /// read-of-stack-slot-before-initialization bugs in Wasmtime's compiler.
    ///
    /// Stack zeroing can be a costly operation in highly concurrent
    /// environments due to modifications of the virtual address space requiring
    /// process-wide synchronization. It can also be costly in `no-std`
    /// environments that must manually zero memory, and cannot rely on an OS
    /// and virtual memory to provide zeroed pages.
    ///
    /// This option defaults to `false`.
    ///
    /// [`call_async`]: crate::TypedFunc::call_async
    #[cfg(feature = "async")]
    pub fn async_stack_zeroing(&mut self, enable: bool) -> &mut Self {
        self.async_stack_zeroing = enable;
        self
    }

    /// Explicitly enables (and un-disables) a given set of [`WasmFeatures`].
    ///
    /// Note: this is a low-level method that does not necessarily imply that
    /// wasmtime _supports_ a feature. It should only be used to _disable_
    /// features that callers want to be rejected by the parser or _enable_
    /// features callers are certain that the current configuration of wasmtime
    /// supports.
    ///
    /// Feature validation is deferred until an engine is being built, thus by
    /// enabling features here a caller may cause
    /// [`Engine::new`](crate::Engine::new) to fail later, if the feature
    /// configuration isn't supported.
    pub fn wasm_features(&mut self, flag: WasmFeatures, enable: bool) -> &mut Self {
        self.enabled_features.set(flag, enable);
        self.disabled_features.set(flag, !enable);
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
    /// This is `true` by default except when the Winch compiler is enabled.
    ///
    /// [WebAssembly tail calls proposal]: https://github.com/WebAssembly/tail-call
    pub fn wasm_tail_call(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::TAIL_CALL, enable);
        self
    }

    /// Configures whether the WebAssembly custom-page-sizes proposal will be
    /// enabled for compilation or not.
    ///
    /// The [WebAssembly custom-page-sizes proposal] allows a memory to
    /// customize its page sizes. By default, Wasm page sizes are 64KiB
    /// large. This proposal allows the memory to opt into smaller page sizes
    /// instead, allowing Wasm to run in environments with less than 64KiB RAM
    /// available, for example.
    ///
    /// Note that the page size is part of the memory's type, and because
    /// different memories may have different types, they may also have
    /// different page sizes.
    ///
    /// Currently the only valid page sizes are 64KiB (the default) and 1
    /// byte. Future extensions may relax this constraint and allow all powers
    /// of two.
    ///
    /// Support for this proposal is disabled by default.
    ///
    /// [WebAssembly custom-page-sizes proposal]: https://github.com/WebAssembly/custom-page-sizes
    pub fn wasm_custom_page_sizes(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CUSTOM_PAGE_SIZES, enable);
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
    /// The default value for this option is whether the `threads`
    /// crate feature of Wasmtime is enabled or not. By default this crate
    /// feature is enabled.
    ///
    /// [threads]: https://github.com/webassembly/threads
    /// [wasi-threads]: https://github.com/webassembly/wasi-threads
    #[cfg(feature = "threads")]
    pub fn wasm_threads(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::THREADS, enable);
        self
    }

    /// Configures whether the WebAssembly [shared-everything-threads] proposal
    /// will be enabled for compilation.
    ///
    /// This feature gates extended use of the `shared` attribute on items other
    /// than memories, extra atomic instructions, and new component model
    /// intrinsics for spawning threads. It depends on the
    /// [`wasm_threads`][Self::wasm_threads] being enabled.
    ///
    /// [shared-everything-threads]:
    ///     https://github.com/webassembly/shared-everything-threads
    pub fn wasm_shared_everything_threads(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::SHARED_EVERYTHING_THREADS, enable);
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
    pub fn wasm_reference_types(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::REFERENCE_TYPES, enable);
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
    pub fn wasm_function_references(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::FUNCTION_REFERENCES, enable);
        self
    }

    /// Configures whether the [WebAssembly wide-arithmetic][proposal] will be
    /// enabled for compilation.
    ///
    /// This feature is `false` by default.
    ///
    /// [proposal]: https://github.com/WebAssembly/wide-arithmetic
    pub fn wasm_wide_arithmetic(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::WIDE_ARITHMETIC, enable);
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
    pub fn wasm_gc(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::GC, enable);
        self
    }

    /// Configures whether the WebAssembly SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly SIMD proposal][proposal]. This feature gates items such
    /// as the `v128` type and all of its operators being in a module. Note that
    /// this does not enable the [relaxed simd proposal].
    ///
    /// **Note**
    ///
    /// On x86_64 platforms the base CPU feature requirement for SIMD
    /// is SSE2 for the Cranelift compiler and AVX for the Winch compiler.
    ///
    /// This is `true` by default.
    ///
    /// [proposal]: https://github.com/webassembly/simd
    /// [relaxed simd proposal]: https://github.com/WebAssembly/relaxed-simd
    pub fn wasm_simd(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::SIMD, enable);
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
        self.wasm_features(WasmFeatures::RELAXED_SIMD, enable);
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
        self.wasm_features(WasmFeatures::BULK_MEMORY, enable);
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
        self.wasm_features(WasmFeatures::MULTI_VALUE, enable);
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
        self.wasm_features(WasmFeatures::MULTI_MEMORY, enable);
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
        self.wasm_features(WasmFeatures::MEMORY64, enable);
        self
    }

    /// Configures whether the WebAssembly extended-const [proposal] will
    /// be enabled for compilation.
    ///
    /// This is `true` by default.
    ///
    /// [proposal]: https://github.com/webassembly/extended-const
    pub fn wasm_extended_const(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::EXTENDED_CONST, enable);
        self
    }

    /// Configures whether the [WebAssembly stack switching
    /// proposal][proposal] will be enabled for compilation.
    ///
    /// This feature gates the use of control tags.
    ///
    /// This feature depends on the `function_reference_types` and
    /// `exceptions` features.
    ///
    /// This feature is `false` by default.
    ///
    /// # Errors
    ///
    /// [proposal]: https://github.com/webassembly/stack-switching
    pub fn wasm_stack_switching(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::STACK_SWITCHING, enable);
        self
    }

    /// Configures whether the WebAssembly component-model [proposal] will
    /// be enabled for compilation.
    ///
    /// This flag can be used to blanket disable all components within Wasmtime.
    /// Otherwise usage of components requires statically using
    /// [`Component`](crate::component::Component) instead of
    /// [`Module`](crate::Module) for example anyway.
    ///
    /// The default value for this option is whether the `component-model`
    /// crate feature of Wasmtime is enabled or not. By default this crate
    /// feature is enabled.
    ///
    /// [proposal]: https://github.com/webassembly/component-model
    #[cfg(feature = "component-model")]
    pub fn wasm_component_model(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::COMPONENT_MODEL, enable);
        self
    }

    /// Configures whether components support the async ABI [proposal] for
    /// lifting and lowering functions, as well as `stream`, `future`, and
    /// `error-context` types.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    ///
    /// [proposal]:
    ///     https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md
    #[cfg(feature = "component-model-async")]
    pub fn wasm_component_model_async(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_ASYNC, enable);
        self
    }

    /// This corresponds to the 🚝 emoji in the component model specification.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    ///
    /// [proposal]:
    ///     https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md
    #[cfg(feature = "component-model-async")]
    pub fn wasm_component_model_async_builtins(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_ASYNC_BUILTINS, enable);
        self
    }

    /// This corresponds to the 🚟 emoji in the component model specification.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    ///
    /// [proposal]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md
    #[cfg(feature = "component-model-async")]
    pub fn wasm_component_model_async_stackful(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_ASYNC_STACKFUL, enable);
        self
    }

    /// This corresponds to the 🧵 emoji in the component model specification.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    ///
    /// [proposal]:
    ///     https://github.com/WebAssembly/component-model/pull/557
    #[cfg(feature = "component-model-async")]
    pub fn wasm_component_model_threading(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_THREADING, enable);
        self
    }

    /// This corresponds to the 📝 emoji in the component model specification.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    ///
    /// [proposal]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md
    #[cfg(feature = "component-model")]
    pub fn wasm_component_model_error_context(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_ERROR_CONTEXT, enable);
        self
    }

    /// Configures whether the [GC extension to the component-model
    /// proposal][proposal] is enabled or not.
    ///
    /// This corresponds to the 🛸 emoji in the component model specification.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    ///
    /// [proposal]: https://github.com/WebAssembly/component-model/issues/525
    #[cfg(feature = "component-model")]
    pub fn wasm_component_model_gc(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_GC, enable);
        self
    }

    /// This corresponds to the 🔧 emoji in the component model specification.
    ///
    /// Please note that Wasmtime's support for this feature is _very_
    /// incomplete.
    #[cfg(feature = "component-model")]
    pub fn wasm_component_model_fixed_length_lists(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::CM_FIXED_SIZE_LIST, enable);
        self
    }

    /// Configures whether the [Exception-handling proposal][proposal] is enabled or not.
    ///
    /// [proposal]: https://github.com/WebAssembly/exception-handling
    #[cfg(feature = "gc")]
    pub fn wasm_exceptions(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::EXCEPTIONS, enable);
        self
    }

    #[doc(hidden)] // FIXME(#3427) - if/when implemented then un-hide this
    #[deprecated = "This configuration option only exists for internal \
                    usage with the spec testsuite. It may be removed at \
                    any time and without warning. Do not rely on it!"]
    pub fn wasm_legacy_exceptions(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::LEGACY_EXCEPTIONS, enable);
        self
    }

    /// Configures which compilation strategy will be used for wasm modules.
    ///
    /// This method can be used to configure which compiler is used for wasm
    /// modules, and for more documentation consult the [`Strategy`] enumeration
    /// and its documentation.
    ///
    /// The default value for this is `Strategy::Auto`.
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn strategy(&mut self, strategy: Strategy) -> &mut Self {
        self.compiler_config_mut().strategy = strategy.not_auto();
        self
    }

    /// Configures which garbage collector will be used for Wasm modules.
    ///
    /// This method can be used to configure which garbage collector
    /// implementation is used for Wasm modules. For more documentation, consult
    /// the [`Collector`] enumeration and its documentation.
    ///
    /// The default value for this is `Collector::Auto`.
    #[cfg(feature = "gc")]
    pub fn collector(&mut self, collector: Collector) -> &mut Self {
        self.collector = collector;
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
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn cranelift_debug_verifier(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler_config_mut()
            .settings
            .insert("enable_verifier".to_string(), val.to_string());
        self
    }

    /// Configures whether extra debug checks are inserted into
    /// Wasmtime-generated code by Cranelift.
    ///
    /// The default value for this is `false`
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn cranelift_wasmtime_debug_checks(&mut self, enable: bool) -> &mut Self {
        unsafe { self.cranelift_flag_set("wasmtime_debug_checks", &enable.to_string()) }
    }

    /// Configures the Cranelift code generator optimization level.
    ///
    /// When the Cranelift code generator is used you can configure the
    /// optimization level used for generated code in a few various ways. For
    /// more information see the documentation of [`OptLevel`].
    ///
    /// The default value for this is `OptLevel::Speed`.
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn cranelift_opt_level(&mut self, level: OptLevel) -> &mut Self {
        let val = match level {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        };
        self.compiler_config_mut()
            .settings
            .insert("opt_level".to_string(), val.to_string());
        self
    }

    /// Configures the regalloc algorithm used by the Cranelift code generator.
    ///
    /// Cranelift can select any of several register allocator algorithms. Each
    /// of these algorithms generates correct code, but they represent different
    /// tradeoffs between compile speed (how expensive the compilation process
    /// is) and run-time speed (how fast the generated code runs).
    /// For more information see the documentation of [`RegallocAlgorithm`].
    ///
    /// The default value for this is `RegallocAlgorithm::Backtracking`.
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn cranelift_regalloc_algorithm(&mut self, algo: RegallocAlgorithm) -> &mut Self {
        let val = match algo {
            RegallocAlgorithm::Backtracking => "backtracking",
            RegallocAlgorithm::SinglePass => "single_pass",
        };
        self.compiler_config_mut()
            .settings
            .insert("regalloc_algorithm".to_string(), val.to_string());
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
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn cranelift_nan_canonicalization(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler_config_mut()
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
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn cranelift_pcc(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler_config_mut()
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
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub unsafe fn cranelift_flag_enable(&mut self, flag: &str) -> &mut Self {
        self.compiler_config_mut().flags.insert(flag.to_string());
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
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub unsafe fn cranelift_flag_set(&mut self, name: &str, value: &str) -> &mut Self {
        self.compiler_config_mut()
            .settings
            .insert(name.to_string(), value.to_string());
        self
    }

    /// Set a custom [`Cache`].
    ///
    /// To load a cache configuration from a file, use [`Cache::from_file`]. Otherwise, you can
    /// create a new cache config using [`CacheConfig::new`] and passing that to [`Cache::new`].
    ///
    /// If you want to disable the cache, you can call this method with `None`.
    ///
    /// By default, new configs do not have caching enabled.
    /// Every call to [`Module::new(my_wasm)`][crate::Module::new] will recompile `my_wasm`,
    /// even when it is unchanged, unless an enabled `CacheConfig` is provided.
    ///
    /// This method is only available when the `cache` feature of this crate is
    /// enabled.
    ///
    /// [docs]: https://bytecodealliance.github.io/wasmtime/cli-cache.html
    #[cfg(feature = "cache")]
    pub fn cache(&mut self, cache: Option<Cache>) -> &mut Self {
        self.cache = cache;
        self
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
    pub fn with_host_stack(&mut self, stack_creator: Arc<dyn StackCreator>) -> &mut Self {
        self.stack_creator = Some(Arc::new(StackCreatorProxy(stack_creator)));
        self
    }

    /// Sets a custom executable-memory publisher.
    ///
    /// Custom executable-memory publishers are hooks that allow
    /// Wasmtime to make certain regions of memory executable when
    /// loading precompiled modules or compiling new modules
    /// in-process. In most modern operating systems, memory allocated
    /// for heap usage is readable and writable by default but not
    /// executable. To jump to machine code stored in that memory, we
    /// need to make it executable. For security reasons, we usually
    /// also make it read-only at the same time, so the executing code
    /// can't be modified later.
    ///
    /// By default, Wasmtime will use the appropriate system calls on
    /// the host platform for this work. However, it also allows
    /// plugging in a custom implementation via this configuration
    /// option. This may be useful on custom or `no_std` platforms,
    /// for example, especially where virtual memory is not otherwise
    /// used by Wasmtime (no `signals-and-traps` feature).
    #[cfg(feature = "runtime")]
    pub fn with_custom_code_memory(
        &mut self,
        custom_code_memory: Option<Arc<dyn CustomCodeMemory>>,
    ) -> &mut Self {
        self.custom_code_memory = custom_code_memory;
        self
    }

    /// Sets the instance allocation strategy to use.
    ///
    /// This is notably used in conjunction with
    /// [`InstanceAllocationStrategy::Pooling`] and [`PoolingAllocationConfig`].
    pub fn allocation_strategy(
        &mut self,
        strategy: impl Into<InstanceAllocationStrategy>,
    ) -> &mut Self {
        self.allocation_strategy = strategy.into();
        self
    }

    /// Specifies the capacity of linear memories, in bytes, in their initial
    /// allocation.
    ///
    /// > Note: this value has important performance ramifications, be sure to
    /// > benchmark when setting this to a non-default value and read over this
    /// > documentation.
    ///
    /// This function will change the size of the initial memory allocation made
    /// for linear memories. This setting is only applicable when the initial
    /// size of a linear memory is below this threshold. Linear memories are
    /// allocated in the virtual address space of the host process with OS APIs
    /// such as `mmap` and this setting affects how large the allocation will
    /// be.
    ///
    /// ## Background: WebAssembly Linear Memories
    ///
    /// WebAssembly linear memories always start with a minimum size and can
    /// possibly grow up to a maximum size. The minimum size is always specified
    /// in a WebAssembly module itself and the maximum size can either be
    /// optionally specified in the module or inherently limited by the index
    /// type. For example for this module:
    ///
    /// ```wasm
    /// (module
    ///     (memory $a 4)
    ///     (memory $b 4096 4096 (pagesize 1))
    ///     (memory $c i64 10)
    /// )
    /// ```
    ///
    /// * Memory `$a` initially allocates 4 WebAssembly pages (256KiB) and can
    ///   grow up to 4GiB, the limit of the 32-bit index space.
    /// * Memory `$b` initially allocates 4096 WebAssembly pages, but in this
    ///   case its page size is 1, so it's 4096 bytes. Memory can also grow no
    ///   further meaning that it will always be 4096 bytes.
    /// * Memory `$c` is a 64-bit linear memory which starts with 640KiB of
    ///   memory and can theoretically grow up to 2^64 bytes, although most
    ///   hosts will run out of memory long before that.
    ///
    /// All operations on linear memories done by wasm are required to be
    /// in-bounds. Any access beyond the end of a linear memory is considered a
    /// trap.
    ///
    /// ## What this setting affects: Virtual Memory
    ///
    /// This setting is used to configure the behavior of the size of the linear
    /// memory allocation performed for each of these memories. For example the
    /// initial linear memory allocation looks like this:
    ///
    /// ```text
    ///              memory_reservation
    ///                    |
    ///          ◄─────────┴────────────────►
    /// ┌───────┬─────────┬──────────────────┬───────┐
    /// │ guard │ initial │ ... capacity ... │ guard │
    /// └───────┴─────────┴──────────────────┴───────┘
    ///  ◄──┬──►                              ◄──┬──►
    ///     │                                    │
    ///     │                             memory_guard_size
    ///     │
    ///     │
    ///  memory_guard_size (if guard_before_linear_memory)
    /// ```
    ///
    /// Memory in the `initial` range is accessible to the instance and can be
    /// read/written by wasm code. Memory in the `guard` regions is never
    /// accessible to wasm code and memory in `capacity` is initially
    /// inaccessible but may become accessible through `memory.grow` instructions
    /// for example.
    ///
    /// This means that this setting is the size of the initial chunk of virtual
    /// memory that a linear memory may grow into.
    ///
    /// ## What this setting affects: Runtime Speed
    ///
    /// This is a performance-sensitive setting which is taken into account
    /// during the compilation process of a WebAssembly module. For example if a
    /// 32-bit WebAssembly linear memory has a `memory_reservation` size of 4GiB
    /// then bounds checks can be elided because `capacity` will be guaranteed
    /// to be unmapped for all addressable bytes that wasm can access (modulo a
    /// few details).
    ///
    /// If `memory_reservation` was something smaller like 256KiB then that
    /// would have a much smaller impact on virtual memory but the compile code
    /// would then need to have explicit bounds checks to ensure that
    /// loads/stores are in-bounds.
    ///
    /// The goal of this setting is to enable skipping bounds checks in most
    /// modules by default. Some situations which require explicit bounds checks
    /// though are:
    ///
    /// * When `memory_reservation` is smaller than the addressable size of the
    ///   linear memory. For example if 64-bit linear memories always need
    ///   bounds checks as they can address the entire virtual address spacce.
    ///   For 32-bit linear memories a `memory_reservation` minimum size of 4GiB
    ///   is required to elide bounds checks.
    ///
    /// * When linear memories have a page size of 1 then bounds checks are
    ///   required. In this situation virtual memory can't be relied upon
    ///   because that operates at the host page size granularity where wasm
    ///   requires a per-byte level granularity.
    ///
    /// * Configuration settings such as [`Config::signals_based_traps`] can be
    ///   used to disable the use of signal handlers and virtual memory so
    ///   explicit bounds checks are required.
    ///
    /// * When [`Config::memory_guard_size`] is too small a bounds check may be
    ///   required. For 32-bit wasm addresses are actually 33-bit effective
    ///   addresses because loads/stores have a 32-bit static offset to add to
    ///   the dynamic 32-bit address. If the static offset is larger than the
    ///   size of the guard region then an explicit bounds check is required.
    ///
    /// ## What this setting affects: Memory Growth Behavior
    ///
    /// In addition to affecting bounds checks emitted in compiled code this
    /// setting also affects how WebAssembly linear memories are grown. The
    /// `memory.grow` instruction can be used to make a linear memory larger and
    /// this is also affected by APIs such as
    /// [`Memory::grow`](crate::Memory::grow).
    ///
    /// In these situations when the amount being grown is small enough to fit
    /// within the remaining capacity then the linear memory doesn't have to be
    /// moved at runtime. If the capacity runs out though then a new linear
    /// memory allocation must be made and the contents of linear memory is
    /// copied over.
    ///
    /// For example here's a situation where a copy happens:
    ///
    /// * The `memory_reservation` setting is configured to 128KiB.
    /// * A WebAssembly linear memory starts with a single 64KiB page.
    /// * This memory can be grown by one page to contain the full 128KiB of
    ///   memory.
    /// * If grown by one more page, though, then a 192KiB allocation must be
    ///   made and the previous 128KiB of contents are copied into the new
    ///   allocation.
    ///
    /// This growth behavior can have a significant performance impact if lots
    /// of data needs to be copied on growth. Conversely if memory growth never
    /// needs to happen because the capacity will always be large enough then
    /// optimizations can be applied to cache the base pointer of linear memory.
    ///
    /// When memory is grown then the
    /// [`Config::memory_reservation_for_growth`] is used for the new
    /// memory allocation to have memory to grow into.
    ///
    /// When using the pooling allocator via [`PoolingAllocationConfig`] then
    /// memories are never allowed to move so requests for growth are instead
    /// rejected with an error.
    ///
    /// ## When this setting is not used
    ///
    /// This setting is ignored and unused when the initial size of linear
    /// memory is larger than this threshold. For example if this setting is set
    /// to 1MiB but a wasm module requires a 2MiB minimum allocation then this
    /// setting is ignored. In this situation the minimum size of memory will be
    /// allocated along with [`Config::memory_reservation_for_growth`]
    /// after it to grow into.
    ///
    /// That means that this value can be set to zero. That can be useful in
    /// benchmarking to see the overhead of bounds checks for example.
    /// Additionally it can be used to minimize the virtual memory allocated by
    /// Wasmtime.
    ///
    /// ## Default Value
    ///
    /// The default value for this property depends on the host platform. For
    /// 64-bit platforms there's lots of address space available, so the default
    /// configured here is 4GiB. When coupled with the default size of
    /// [`Config::memory_guard_size`] this means that 32-bit WebAssembly linear
    /// memories with 64KiB page sizes will skip almost all bounds checks by
    /// default.
    ///
    /// For 32-bit platforms this value defaults to 10MiB. This means that
    /// bounds checks will be required on 32-bit platforms.
    pub fn memory_reservation(&mut self, bytes: u64) -> &mut Self {
        self.tunables.memory_reservation = Some(bytes);
        self
    }

    /// Indicates whether linear memories may relocate their base pointer at
    /// runtime.
    ///
    /// WebAssembly linear memories either have a maximum size that's explicitly
    /// listed in the type of a memory or inherently limited by the index type
    /// of the memory (e.g. 4GiB for 32-bit linear memories). Depending on how
    /// the linear memory is allocated (see [`Config::memory_reservation`]) it
    /// may be necessary to move the memory in the host's virtual address space
    /// during growth. This option controls whether this movement is allowed or
    /// not.
    ///
    /// An example of a linear memory needing to move is when
    /// [`Config::memory_reservation`] is 0 then a linear memory will be
    /// allocated as the minimum size of the memory plus
    /// [`Config::memory_reservation_for_growth`]. When memory grows beyond the
    /// reservation for growth then the memory needs to be relocated.
    ///
    /// When this option is set to `false` then it can have a number of impacts
    /// on how memories work at runtime:
    ///
    /// * Modules can be compiled with static knowledge the base pointer of
    ///   linear memory never changes to enable optimizations such as
    ///   loop invariant code motion (hoisting the base pointer out of a loop).
    ///
    /// * Memories cannot grow in excess of their original allocation. This
    ///   means that [`Config::memory_reservation`] and
    ///   [`Config::memory_reservation_for_growth`] may need tuning to ensure
    ///   the memory configuration works at runtime.
    ///
    /// The default value for this option is `true`.
    pub fn memory_may_move(&mut self, enable: bool) -> &mut Self {
        self.tunables.memory_may_move = Some(enable);
        self
    }

    /// Configures the size, in bytes, of the guard region used at the end of a
    /// linear memory's address space reservation.
    ///
    /// > Note: this value has important performance ramifications, be sure to
    /// > understand what this value does before tweaking it and benchmarking.
    ///
    /// This setting controls how many bytes are guaranteed to be unmapped after
    /// the virtual memory allocation of a linear memory. When
    /// combined with sufficiently large values of
    /// [`Config::memory_reservation`] (e.g. 4GiB for 32-bit linear memories)
    /// then a guard region can be used to eliminate bounds checks in generated
    /// code.
    ///
    /// This setting additionally can be used to help deduplicate bounds checks
    /// in code that otherwise requires bounds checks. For example with a 4KiB
    /// guard region then a 64-bit linear memory which accesses addresses `x+8`
    /// and `x+16` only needs to perform a single bounds check on `x`. If that
    /// bounds check passes then the offset is guaranteed to either reside in
    /// linear memory or the guard region, resulting in deterministic behavior
    /// either way.
    ///
    /// ## How big should the guard be?
    ///
    /// In general, like with configuring [`Config::memory_reservation`], you
    /// probably don't want to change this value from the defaults. Removing
    /// bounds checks is dependent on a number of factors where the size of the
    /// guard region is only one piece of the equation. Other factors include:
    ///
    /// * [`Config::memory_reservation`]
    /// * The index type of the linear memory (e.g. 32-bit or 64-bit)
    /// * The page size of the linear memory
    /// * Other settings such as [`Config::signals_based_traps`]
    ///
    /// Embeddings using virtual memory almost always want at least some guard
    /// region, but otherwise changes from the default should be profiled
    /// locally to see the performance impact.
    ///
    /// ## Default
    ///
    /// The default value for this property is 32MiB on 64-bit platforms. This
    /// allows eliminating almost all bounds checks on loads/stores with an
    /// immediate offset of less than 32MiB. On 32-bit platforms this defaults
    /// to 64KiB.
    pub fn memory_guard_size(&mut self, bytes: u64) -> &mut Self {
        self.tunables.memory_guard_size = Some(bytes);
        self
    }

    /// Configures the size, in bytes, of the extra virtual memory space
    /// reserved after a linear memory is relocated.
    ///
    /// This setting is used in conjunction with [`Config::memory_reservation`]
    /// to configure what happens after a linear memory is relocated in the host
    /// address space. If the initial size of a linear memory exceeds
    /// [`Config::memory_reservation`] or if it grows beyond that size
    /// throughout its lifetime then this setting will be used.
    ///
    /// When a linear memory is relocated it will initially look like this:
    ///
    /// ```text
    ///            memory.size
    ///                 │
    ///          ◄──────┴─────►
    /// ┌───────┬──────────────┬───────┐
    /// │ guard │  accessible  │ guard │
    /// └───────┴──────────────┴───────┘
    ///                         ◄──┬──►
    ///                            │
    ///                     memory_guard_size
    /// ```
    ///
    /// where `accessible` needs to be grown but there's no more memory to grow
    /// into. A new region of the virtual address space will be allocated that
    /// looks like this:
    ///
    /// ```text
    ///                           memory_reservation_for_growth
    ///                                       │
    ///            memory.size                │
    ///                 │                     │
    ///          ◄──────┴─────► ◄─────────────┴───────────►
    /// ┌───────┬──────────────┬───────────────────────────┬───────┐
    /// │ guard │  accessible  │ .. reserved for growth .. │ guard │
    /// └───────┴──────────────┴───────────────────────────┴───────┘
    ///                                                     ◄──┬──►
    ///                                                        │
    ///                                               memory_guard_size
    /// ```
    ///
    /// This means that up to `memory_reservation_for_growth` bytes can be
    /// allocated again before the entire linear memory needs to be moved again
    /// when another `memory_reservation_for_growth` bytes will be appended to
    /// the size of the allocation.
    ///
    /// Note that this is a currently simple heuristic for optimizing the growth
    /// of dynamic memories, primarily implemented for the memory64 proposal
    /// where the maximum size of memory is larger than 4GiB. This setting is
    /// unlikely to be a one-size-fits-all style approach and if you're an
    /// embedder running into issues with growth and are interested in having
    /// other growth strategies available here please feel free to [open an
    /// issue on the Wasmtime repository][issue]!
    ///
    /// [issue]: https://github.com/bytecodealliance/wasmtime/issues/new
    ///
    /// ## Default
    ///
    /// For 64-bit platforms this defaults to 2GiB, and for 32-bit platforms
    /// this defaults to 1MiB.
    pub fn memory_reservation_for_growth(&mut self, bytes: u64) -> &mut Self {
        self.tunables.memory_reservation_for_growth = Some(bytes);
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
    /// [`Config::memory_guard_size`].
    ///
    /// ## Default
    ///
    /// This value defaults to `true`.
    pub fn guard_before_linear_memory(&mut self, enable: bool) -> &mut Self {
        self.tunables.guard_before_linear_memory = Some(enable);
        self
    }

    /// Indicates whether to initialize tables lazily, so that instantiation
    /// is fast but indirect calls are a little slower. If false, tables
    /// are initialized eagerly during instantiation from any active element
    /// segments that apply to them.
    ///
    /// **Note** Disabling this option is not compatible with the Winch compiler.
    ///
    /// ## Default
    ///
    /// This value defaults to `true`.
    pub fn table_lazy_init(&mut self, table_lazy_init: bool) -> &mut Self {
        self.tunables.table_lazy_init = Some(table_lazy_init);
        self
    }

    /// Configure the version information used in serialized and deserialized [`crate::Module`]s.
    /// This effects the behavior of [`crate::Module::serialize()`], as well as
    /// [`crate::Module::deserialize()`] and related functions.
    ///
    /// The default strategy is to use the wasmtime crate's Cargo package version.
    pub fn module_version(&mut self, strategy: ModuleVersionStrategy) -> Result<&mut Self> {
        match strategy {
            // This case requires special precondition for assertion in SerializedModule::to_bytes
            ModuleVersionStrategy::Custom(ref v) => {
                if v.as_bytes().len() > 255 {
                    bail!("custom module version cannot be more than 255 bytes: {v}");
                }
            }
            _ => {}
        }
        self.module_version = strategy;
        Ok(self)
    }

    /// Configure whether wasmtime should compile a module using multiple
    /// threads.
    ///
    /// Disabling this will result in a single thread being used to compile
    /// the wasm bytecode.
    ///
    /// By default parallel compilation is enabled.
    #[cfg(feature = "parallel-compilation")]
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
        self.tunables.memory_init_cow = Some(enable);
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
    /// the [`Error`](crate::Error) when a trap is raised.
    ///
    /// This option is disabled by default.
    #[cfg(feature = "coredump")]
    pub fn coredump_on_trap(&mut self, enable: bool) -> &mut Self {
        self.coredump_on_trap = enable;
        self
    }

    /// Enables memory error checking for wasm programs.
    ///
    /// This option is disabled by default.
    ///
    /// # Panics
    ///
    /// Panics if this configuration's compiler was [disabled][Config::enable_compiler].
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn wmemcheck(&mut self, enable: bool) -> &mut Self {
        self.wmemcheck = enable;
        self.compiler_config_mut().wmemcheck = enable;
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

    /// Whether to enable function inlining during compilation or not.
    ///
    /// This may result in faster execution at runtime, but adds additional
    /// compilation time. Inlining may also enlarge the size of compiled
    /// artifacts (for example, the size of the result of
    /// [`Engine::precompile_component`](crate::Engine::precompile_component)).
    ///
    /// Inlining is not supported by all of Wasmtime's compilation strategies;
    /// currently, it only Cranelift supports it. This setting will be ignored
    /// when using a compilation strategy that does not support inlining, like
    /// Winch.
    ///
    /// Note that inlining is still somewhat experimental at the moment (as of
    /// the Wasmtime version 36).
    pub fn compiler_inlining(&mut self, inlining: bool) -> &mut Self {
        self.tunables.inlining = Some(inlining);
        self
    }

    /// Returns the set of features that the currently selected compiler backend
    /// does not support at all and may panic on.
    ///
    /// Wasmtime strives to reject unknown modules or unsupported modules with
    /// first-class errors instead of panics. Not all compiler backends have the
    /// same level of feature support on all platforms as well. This method
    /// returns a set of features that the currently selected compiler
    /// configuration is known to not support and may panic on. This acts as a
    /// first-level filter on incoming wasm modules/configuration to fail-fast
    /// instead of panicking later on.
    ///
    /// Note that if a feature is not listed here it does not mean that the
    /// backend fully supports the proposal. Instead that means that the backend
    /// doesn't ever panic on the proposal, but errors during compilation may
    /// still be returned. This means that features listed here are definitely
    /// not supported at all, but features not listed here may still be
    /// partially supported. For example at the time of this writing the Winch
    /// backend partially supports simd so it's not listed here. Winch doesn't
    /// fully support simd but unimplemented instructions just return errors.
    fn compiler_panicking_wasm_features(&self) -> WasmFeatures {
        // First we compute the set of features that Wasmtime itself knows;
        // this is a sort of "maximal set" that we invert to create a set
        // of features we _definitely can't support_ because wasmtime
        // has never heard of them.
        let features_known_to_wasmtime = WasmFeatures::empty()
            | WasmFeatures::MUTABLE_GLOBAL
            | WasmFeatures::SATURATING_FLOAT_TO_INT
            | WasmFeatures::SIGN_EXTENSION
            | WasmFeatures::REFERENCE_TYPES
            | WasmFeatures::CALL_INDIRECT_OVERLONG
            | WasmFeatures::MULTI_VALUE
            | WasmFeatures::BULK_MEMORY
            | WasmFeatures::BULK_MEMORY_OPT
            | WasmFeatures::SIMD
            | WasmFeatures::RELAXED_SIMD
            | WasmFeatures::THREADS
            | WasmFeatures::SHARED_EVERYTHING_THREADS
            | WasmFeatures::TAIL_CALL
            | WasmFeatures::FLOATS
            | WasmFeatures::MULTI_MEMORY
            | WasmFeatures::EXCEPTIONS
            | WasmFeatures::MEMORY64
            | WasmFeatures::EXTENDED_CONST
            | WasmFeatures::COMPONENT_MODEL
            | WasmFeatures::FUNCTION_REFERENCES
            | WasmFeatures::GC
            | WasmFeatures::CUSTOM_PAGE_SIZES
            | WasmFeatures::GC_TYPES
            | WasmFeatures::STACK_SWITCHING
            | WasmFeatures::WIDE_ARITHMETIC
            | WasmFeatures::CM_ASYNC
            | WasmFeatures::CM_ASYNC_STACKFUL
            | WasmFeatures::CM_ASYNC_BUILTINS
            | WasmFeatures::CM_THREADING
            | WasmFeatures::CM_ERROR_CONTEXT
            | WasmFeatures::CM_GC
            | WasmFeatures::CM_FIXED_SIZE_LIST;

        #[allow(unused_mut, reason = "easier to avoid #[cfg]")]
        let mut unsupported = !features_known_to_wasmtime;

        #[cfg(any(feature = "cranelift", feature = "winch"))]
        match self.compiler_config.as_ref().and_then(|c| c.strategy) {
            None | Some(Strategy::Cranelift) => {
                // Pulley at this time fundamentally doesn't support the
                // `threads` proposal, notably shared memory, because Rust can't
                // safely implement loads/stores in the face of shared memory.
                // Stack switching is not implemented, either.
                if self.compiler_target().is_pulley() {
                    unsupported |= WasmFeatures::THREADS;
                    unsupported |= WasmFeatures::STACK_SWITCHING;
                }

                use target_lexicon::*;
                match self.compiler_target() {
                    Triple {
                        architecture: Architecture::X86_64 | Architecture::X86_64h,
                        operating_system:
                            OperatingSystem::Linux
                            | OperatingSystem::MacOSX(_)
                            | OperatingSystem::Darwin(_),
                        ..
                    } => {
                        // Stack switching supported on (non-Pulley) Cranelift.
                    }

                    _ => {
                        // On platforms other than x64 Unix-like, we don't
                        // support stack switching.
                        unsupported |= WasmFeatures::STACK_SWITCHING;
                    }
                }
            }
            Some(Strategy::Winch) => {
                unsupported |= WasmFeatures::GC
                    | WasmFeatures::FUNCTION_REFERENCES
                    | WasmFeatures::RELAXED_SIMD
                    | WasmFeatures::TAIL_CALL
                    | WasmFeatures::GC_TYPES
                    | WasmFeatures::EXCEPTIONS
                    | WasmFeatures::LEGACY_EXCEPTIONS
                    | WasmFeatures::STACK_SWITCHING
                    | WasmFeatures::CM_ASYNC;
                match self.compiler_target().architecture {
                    target_lexicon::Architecture::Aarch64(_) => {
                        unsupported |= WasmFeatures::THREADS;
                        unsupported |= WasmFeatures::WIDE_ARITHMETIC;
                    }

                    // Winch doesn't support other non-x64 architectures at this
                    // time either but will return an first-class error for
                    // them.
                    _ => {}
                }
            }
            Some(Strategy::Auto) => unreachable!(),
        }
        unsupported
    }

    /// Calculates the set of features that are enabled for this `Config`.
    ///
    /// This method internally will start with the an empty set of features to
    /// avoid being tied to wasmparser's defaults. Next Wasmtime's set of
    /// default features are added to this set, some of which are conditional
    /// depending on crate features. Finally explicitly requested features via
    /// `wasm_*` methods on `Config` are applied. Everything is then validated
    /// later in `Config::validate`.
    fn features(&self) -> WasmFeatures {
        // Wasmtime by default supports all of the wasm 2.0 version of the
        // specification.
        let mut features = WasmFeatures::WASM2;

        // On-by-default features that wasmtime has. Note that these are all
        // subject to the criteria at
        // https://docs.wasmtime.dev/contributing-implementing-wasm-proposals.html
        // and
        // https://docs.wasmtime.dev/stability-wasm-proposals.html
        features |= WasmFeatures::MULTI_MEMORY;
        features |= WasmFeatures::RELAXED_SIMD;
        features |= WasmFeatures::TAIL_CALL;
        features |= WasmFeatures::EXTENDED_CONST;
        features |= WasmFeatures::MEMORY64;
        // NB: if you add a feature above this line please double-check
        // https://docs.wasmtime.dev/stability-wasm-proposals.html
        // to ensure all requirements are met and/or update the documentation
        // there too.

        // Set some features to their conditionally-enabled defaults depending
        // on crate compile-time features.
        features.set(WasmFeatures::GC_TYPES, cfg!(feature = "gc"));
        features.set(WasmFeatures::THREADS, cfg!(feature = "threads"));
        features.set(
            WasmFeatures::COMPONENT_MODEL,
            cfg!(feature = "component-model"),
        );

        // From the default set of proposals remove any that the current
        // compiler backend may panic on if the module contains them.
        features = features & !self.compiler_panicking_wasm_features();

        // After wasmtime's defaults are configured then factor in user requests
        // and disable/enable features. Note that the enable/disable sets should
        // be disjoint.
        debug_assert!((self.enabled_features & self.disabled_features).is_empty());
        features &= !self.disabled_features;
        features |= self.enabled_features;

        features
    }

    /// Returns the configured compiler target for this `Config`.
    pub(crate) fn compiler_target(&self) -> target_lexicon::Triple {
        // If a target is explicitly configured, always use that.
        if let Some(target) = self.target.clone() {
            return target;
        }

        // If the `build.rs` script determined that this platform uses pulley by
        // default, then use Pulley.
        if cfg!(default_target_pulley) {
            return target_lexicon::Triple::pulley_host();
        }

        // And at this point the target is for sure the host.
        target_lexicon::Triple::host()
    }

    pub(crate) fn validate(&self) -> Result<(Tunables, WasmFeatures)> {
        let features = self.features();

        // First validate that the selected compiler backend and configuration
        // supports the set of `features` that are enabled. This will help
        // provide more first class errors instead of panics about unsupported
        // features and configurations.
        let unsupported = features & self.compiler_panicking_wasm_features();
        if !unsupported.is_empty() {
            for flag in WasmFeatures::FLAGS.iter() {
                if !unsupported.contains(*flag.value()) {
                    continue;
                }
                bail!(
                    "the wasm_{} feature is not supported on this compiler configuration",
                    flag.name().to_lowercase()
                );
            }

            panic!("should have returned an error by now")
        }

        #[cfg(any(feature = "async", feature = "stack-switching"))]
        if self.max_wasm_stack > self.async_stack_size {
            bail!("max_wasm_stack size cannot exceed the async_stack_size");
        }
        if self.max_wasm_stack == 0 {
            bail!("max_wasm_stack size cannot be zero");
        }
        if !cfg!(feature = "wmemcheck") && self.wmemcheck {
            bail!("wmemcheck (memory checker) was requested but is not enabled in this build");
        }

        if !cfg!(feature = "gc") && features.gc_types() {
            bail!("support for GC was disabled at compile time")
        }

        if !cfg!(feature = "gc") && features.contains(WasmFeatures::EXCEPTIONS) {
            bail!("exceptions support requires garbage collection (GC) to be enabled in the build");
        }

        let mut tunables = Tunables::default_for_target(&self.compiler_target())?;

        // By default this is enabled with the Cargo feature, and if the feature
        // is missing this is disabled.
        tunables.concurrency_support = cfg!(feature = "component-model-async");

        // If no target is explicitly specified then further refine `tunables`
        // for the configuration of this host depending on what platform
        // features were found available at compile time. This means that anyone
        // cross-compiling for a customized host will need to further refine
        // compilation options.
        if self.target.is_none() {
            // If this platform doesn't have native signals then change some
            // defaults to account for that. Note that VM guards are turned off
            // here because that's primarily a feature of eliding
            // bounds-checks.
            if !cfg!(has_native_signals) {
                tunables.signals_based_traps = cfg!(has_native_signals);
                tunables.memory_guard_size = 0;
            }

            // When virtual memory is not available use slightly different
            // defaults for tunables to be more amenable to `MallocMemory`.
            // Note that these can still be overridden by config options.
            if !cfg!(has_virtual_memory) {
                tunables.memory_reservation = 0;
                tunables.memory_reservation_for_growth = 1 << 20; // 1MB
                tunables.memory_init_cow = false;
            }
        }

        // If guest-debugging is enabled, we must disable
        // signals-based traps. Do this before we process the user's
        // provided tunables settings so we can detect a conflict with
        // an explicit request to use signals-based traps.
        #[cfg(feature = "debug")]
        if self.tunables.debug_guest == Some(true) {
            tunables.signals_based_traps = false;
        }

        self.tunables.configure(&mut tunables);

        // If we're going to compile with winch, we must use the winch calling convention.
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        {
            tunables.winch_callable = self
                .compiler_config
                .as_ref()
                .is_some_and(|c| c.strategy == Some(Strategy::Winch));
        }

        tunables.collector = if features.gc_types() {
            #[cfg(feature = "gc")]
            {
                use wasmtime_environ::Collector as EnvCollector;
                Some(match self.collector.try_not_auto()? {
                    Collector::DeferredReferenceCounting => EnvCollector::DeferredReferenceCounting,
                    Collector::Null => EnvCollector::Null,
                    Collector::Auto => unreachable!(),
                })
            }
            #[cfg(not(feature = "gc"))]
            bail!("cannot use GC types: the `gc` feature was disabled at compile time")
        } else {
            None
        };

        if tunables.debug_guest {
            ensure!(
                cfg!(feature = "debug"),
                "debug instrumentation support was disabled at compile time"
            );
            ensure!(
                !tunables.signals_based_traps,
                "cannot use signals-based traps with guest debugging enabled"
            );
        }

        // Concurrency support is required for some component model features.
        let requires_concurrency = WasmFeatures::CM_ASYNC
            | WasmFeatures::CM_ASYNC_BUILTINS
            | WasmFeatures::CM_ASYNC_STACKFUL
            | WasmFeatures::CM_THREADING
            | WasmFeatures::CM_ERROR_CONTEXT;
        if tunables.concurrency_support && !cfg!(feature = "component-model-async") {
            bail!(
                "concurrency support was requested but was not \
                 compiled into this build of Wasmtime"
            )
        }
        if !tunables.concurrency_support && features.intersects(requires_concurrency) {
            bail!(
                "concurrency support must be enabled to use the component \
                 model async or threading features"
            )
        }

        Ok((tunables, features))
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn build_allocator(
        &self,
        tunables: &Tunables,
    ) -> Result<Box<dyn InstanceAllocator + Send + Sync>> {
        #[cfg(feature = "async")]
        let (stack_size, stack_zeroing) = (self.async_stack_size, self.async_stack_zeroing);

        #[cfg(not(feature = "async"))]
        let (stack_size, stack_zeroing) = (0, false);

        let _ = tunables;

        match &self.allocation_strategy {
            InstanceAllocationStrategy::OnDemand => {
                let mut _allocator = try_new::<Box<_>>(OnDemandInstanceAllocator::new(
                    self.mem_creator.clone(),
                    stack_size,
                    stack_zeroing,
                ))?;
                #[cfg(feature = "async")]
                if let Some(stack_creator) = &self.stack_creator {
                    _allocator.set_stack_creator(stack_creator.clone());
                }
                Ok(_allocator as _)
            }
            #[cfg(feature = "pooling-allocator")]
            InstanceAllocationStrategy::Pooling(config) => {
                let mut config = config.config;
                config.stack_size = stack_size;
                config.async_stack_zeroing = stack_zeroing;
                let allocator = try_new::<Box<_>>(
                    crate::runtime::vm::PoolingInstanceAllocator::new(&config, tunables)?,
                )?;
                Ok(allocator as _)
            }
        }
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn build_gc_runtime(&self) -> Result<Option<Arc<dyn GcRuntime>>> {
        if !self.features().gc_types() {
            return Ok(None);
        }

        #[cfg(not(feature = "gc"))]
        bail!("cannot create a GC runtime: the `gc` feature was disabled at compile time");

        #[cfg(feature = "gc")]
        #[cfg_attr(
            not(any(feature = "gc-null", feature = "gc-drc")),
            expect(unreachable_code, reason = "definitions known to be dummy")
        )]
        {
            Ok(Some(match self.collector.try_not_auto()? {
                #[cfg(feature = "gc-drc")]
                Collector::DeferredReferenceCounting => {
                    try_new::<Arc<_>>(crate::runtime::vm::DrcCollector::default())? as _
                }
                #[cfg(not(feature = "gc-drc"))]
                Collector::DeferredReferenceCounting => unreachable!(),

                #[cfg(feature = "gc-null")]
                Collector::Null => {
                    try_new::<Arc<_>>(crate::runtime::vm::NullCollector::default())? as _
                }
                #[cfg(not(feature = "gc-null"))]
                Collector::Null => unreachable!(),

                Collector::Auto => unreachable!(),
            }))
        }
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn build_profiler(&self) -> Result<Box<dyn ProfilingAgent>> {
        Ok(match self.profiling_strategy {
            ProfilingStrategy::PerfMap => profiling_agent::new_perfmap()?,
            ProfilingStrategy::JitDump => profiling_agent::new_jitdump()?,
            ProfilingStrategy::VTune => profiling_agent::new_vtune()?,
            ProfilingStrategy::None => profiling_agent::new_null(),
            ProfilingStrategy::Pulley => profiling_agent::new_pulley()?,
        })
    }

    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub(crate) fn build_compiler(
        mut self,
        tunables: &mut Tunables,
        features: WasmFeatures,
    ) -> Result<(Self, Box<dyn wasmtime_environ::Compiler>)> {
        let target = self.compiler_target();

        // The target passed to the builders below is an `Option<Triple>` where
        // `None` represents the current host with CPU features inferred from
        // the host's CPU itself. The `target` above is not an `Option`, so
        // switch it to `None` in the case that a target wasn't explicitly
        // specified (which indicates no feature inference) and the target
        // matches the host.
        let target_for_builder =
            if self.target.is_none() && target == target_lexicon::Triple::host() {
                None
            } else {
                Some(target.clone())
            };

        let mut compiler = match self.compiler_config_mut().strategy {
            #[cfg(feature = "cranelift")]
            Some(Strategy::Cranelift) => wasmtime_cranelift::builder(target_for_builder)?,
            #[cfg(not(feature = "cranelift"))]
            Some(Strategy::Cranelift) => bail!("cranelift support not compiled in"),
            #[cfg(feature = "winch")]
            Some(Strategy::Winch) => wasmtime_winch::builder(target_for_builder)?,
            #[cfg(not(feature = "winch"))]
            Some(Strategy::Winch) => bail!("winch support not compiled in"),

            None | Some(Strategy::Auto) => unreachable!(),
        };

        if let Some(path) = &self.compiler_config_mut().clif_dir {
            compiler.clif_dir(path)?;
        }

        // If probestack is enabled for a target, Wasmtime will always use the
        // inline strategy which doesn't require us to define a `__probestack`
        // function or similar.
        self.compiler_config_mut()
            .settings
            .insert("probestack_strategy".into(), "inline".into());

        // We enable stack probing by default on all targets.
        // This is required on Windows because of the way Windows
        // commits its stacks, but it's also a good idea on other
        // platforms to ensure guard pages are hit for large frame
        // sizes.
        self.compiler_config_mut()
            .flags
            .insert("enable_probestack".into());

        // The current wasm multivalue implementation depends on this.
        // FIXME(#9510) handle this in wasmtime-cranelift instead.
        self.compiler_config_mut()
            .flags
            .insert("enable_multi_ret_implicit_sret".into());

        if let Some(unwind_requested) = self.native_unwind_info {
            if !self
                .compiler_config_mut()
                .ensure_setting_unset_or_given("unwind_info", &unwind_requested.to_string())
            {
                bail!(
                    "incompatible settings requested for Cranelift and Wasmtime `unwind-info` settings"
                );
            }
        }

        if target.operating_system == target_lexicon::OperatingSystem::Windows {
            if !self
                .compiler_config_mut()
                .ensure_setting_unset_or_given("unwind_info", "true")
            {
                bail!("`native_unwind_info` cannot be disabled on Windows");
            }
        }

        // We require frame pointers for correct stack walking, which is safety
        // critical in the presence of reference types, and otherwise it is just
        // really bad developer experience to get wrong.
        self.compiler_config_mut()
            .settings
            .insert("preserve_frame_pointers".into(), "true".into());

        if !tunables.signals_based_traps {
            let mut ok = self
                .compiler_config_mut()
                .ensure_setting_unset_or_given("enable_table_access_spectre_mitigation", "false");
            ok = ok
                && self.compiler_config_mut().ensure_setting_unset_or_given(
                    "enable_heap_access_spectre_mitigation",
                    "false",
                );

            // Right now spectre-mitigated bounds checks will load from zero so
            // if host-based signal handlers are disabled then that's a mismatch
            // and doesn't work right now. Fixing this will require more thought
            // of how to implement the bounds check in spectre-only mode.
            if !ok {
                bail!(
                    "when signals-based traps are disabled then spectre \
                     mitigations must also be disabled"
                );
            }
        }

        if features.contains(WasmFeatures::RELAXED_SIMD) && !features.contains(WasmFeatures::SIMD) {
            bail!("cannot disable the simd proposal but enable the relaxed simd proposal");
        }

        if features.contains(WasmFeatures::STACK_SWITCHING) {
            use target_lexicon::OperatingSystem;
            let model = match target.operating_system {
                OperatingSystem::Windows => "update_windows_tib",
                OperatingSystem::Linux
                | OperatingSystem::MacOSX(_)
                | OperatingSystem::Darwin(_) => "basic",
                _ => bail!("stack-switching feature not supported on this platform "),
            };

            if !self
                .compiler_config_mut()
                .ensure_setting_unset_or_given("stack_switch_model", model)
            {
                bail!(
                    "compiler option 'stack_switch_model' must be set to '{model}' on this platform"
                );
            }
        }

        // Apply compiler settings and flags
        compiler.set_tunables(tunables.clone())?;
        for (k, v) in self.compiler_config_mut().settings.iter() {
            compiler.set(k, v)?;
        }
        for flag in self.compiler_config_mut().flags.iter() {
            compiler.enable(flag)?;
        }
        *tunables = compiler.tunables().cloned().unwrap();

        #[cfg(all(feature = "incremental-cache", feature = "cranelift"))]
        if let Some(cache_store) = &self.compiler_config_mut().cache_store {
            compiler.enable_incremental_compilation(cache_store.clone())?;
        }

        compiler.wmemcheck(self.compiler_config_mut().wmemcheck);

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
        self.compiler_config_mut().clif_dir = Some(path.to_path_buf());
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

    /// Configures an embedder-provided function, `detect`, which is used to
    /// determine if an ISA-specific feature is available on the current host.
    ///
    /// This function is used to verify that any features enabled for a compiler
    /// backend, such as AVX support on x86\_64, are also available on the host.
    /// It is undefined behavior to execute an AVX instruction on a host that
    /// doesn't support AVX instructions, for example.
    ///
    /// When the `std` feature is active on this crate then this function is
    /// configured to a default implementation that uses the standard library's
    /// feature detection. When the `std` feature is disabled then there is no
    /// default available and this method must be called to configure a feature
    /// probing function.
    ///
    /// The `detect` function provided is given a string name of an ISA feature.
    /// The function should then return:
    ///
    /// * `Some(true)` - indicates that the feature was found on the host and it
    ///   is supported.
    /// * `Some(false)` - the feature name was recognized but it was not
    ///   detected on the host, for example the CPU is too old.
    /// * `None` - the feature name was not recognized and it's not known
    ///   whether it's on the host or not.
    ///
    /// Feature names passed to `detect` match the same feature name used in the
    /// Rust standard library. For example `"sse4.2"` is used on x86\_64.
    ///
    /// # Unsafety
    ///
    /// This function is `unsafe` because it is undefined behavior to execute
    /// instructions that a host does not support. This means that the result of
    /// `detect` must be correct for memory safe execution at runtime.
    pub unsafe fn detect_host_feature(&mut self, detect: fn(&str) -> Option<bool>) -> &mut Self {
        self.detect_host_feature = Some(detect);
        self
    }

    /// Configures Wasmtime to not use signals-based trap handlers, for example
    /// disables `SIGILL` and `SIGSEGV` handler registration on Unix platforms.
    ///
    /// > **Note:** this option has important performance ramifications, be sure
    /// > to understand the implications. Wasm programs have been measured to
    /// > run up to 2x slower when signals-based traps are disabled.
    ///
    /// Wasmtime will by default leverage signals-based trap handlers (or the
    /// platform equivalent, for example "vectored exception handlers" on
    /// Windows) to make generated code more efficient. For example, when
    /// Wasmtime can use signals-based traps, it can elide explicit bounds
    /// checks for Wasm linear memory accesses, instead relying on virtual
    /// memory guard pages to raise a `SIGSEGV` (on Unix) for out-of-bounds
    /// accesses, which Wasmtime's runtime then catches and handles. Another
    /// example is divide-by-zero: with signals-based traps, Wasmtime can let
    /// the hardware raise a trap when the divisor is zero. Without
    /// signals-based traps, Wasmtime must explicitly emit additional
    /// instructions to check for zero and conditionally branch to a trapping
    /// code path.
    ///
    /// Some environments however may not have access to signal handlers. For
    /// example embedded scenarios may not support virtual memory. Other
    /// environments where Wasmtime is embedded within the surrounding
    /// environment may require that new signal handlers aren't registered due
    /// to the global nature of signal handlers. This option exists to disable
    /// the signal handler registration when required for these scenarios.
    ///
    /// When signals-based trap handlers are disabled, then Wasmtime and its
    /// generated code will *never* rely on segfaults or other
    /// signals. Generated code will be slower because bounds must be explicitly
    /// checked along with other conditions like division by zero.
    ///
    /// The following additional factors can also affect Wasmtime's ability to
    /// elide explicit bounds checks and leverage signals-based traps:
    ///
    /// * The [`Config::memory_reservation`] and [`Config::memory_guard_size`]
    ///   settings
    /// * The index type of the linear memory (e.g. 32-bit or 64-bit)
    /// * The page size of the linear memory
    ///
    /// When this option is disabled, the
    /// `enable_heap_access_spectre_mitigation` and
    /// `enable_table_access_spectre_mitigation` Cranelift settings must also be
    /// disabled. This means that generated code must have spectre mitigations
    /// disabled. This is because spectre mitigations rely on faults from
    /// loading from the null address to implement bounds checks.
    ///
    /// This option defaults to `true`: signals-based trap handlers are enabled
    /// by default.
    ///
    /// > **Note:** Disabling this option is not compatible with the Winch
    /// > compiler.
    pub fn signals_based_traps(&mut self, enable: bool) -> &mut Self {
        self.tunables.signals_based_traps = Some(enable);
        self
    }

    /// Enable/disable GC support in Wasmtime entirely.
    ///
    /// This flag can be used to gate whether GC infrastructure is enabled or
    /// initialized in Wasmtime at all. Wasmtime's GC implementation is required
    /// for the [`Self::wasm_gc`] proposal, [`Self::wasm_function_references`],
    /// and [`Self::wasm_exceptions`] at this time. None of those proposal can
    /// be enabled without also having this option enabled.
    ///
    /// This option defaults to whether the crate `gc` feature is enabled or
    /// not.
    pub fn gc_support(&mut self, enable: bool) -> &mut Self {
        self.wasm_features(WasmFeatures::GC_TYPES, enable)
    }

    /// Explicitly indicate or not whether the host is using a hardware float
    /// ABI on x86 targets.
    ///
    /// This configuration option is only applicable on the
    /// `x86_64-unknown-none` Rust target and has no effect on other host
    /// targets. The `x86_64-unknown-none` Rust target does not support hardware
    /// floats by default and uses a "soft float" implementation and ABI. This
    /// means that `f32`, for example, is passed in a general-purpose register
    /// between functions instead of a floating-point register. This does not
    /// match Cranelift's ABI for `f32` where it's passed in floating-point
    /// registers.  Cranelift does not have support for a "soft float"
    /// implementation where all floating-point operations are lowered to
    /// libcalls.
    ///
    /// This means that for the `x86_64-unknown-none` target the ABI between
    /// Wasmtime's libcalls and the host is incompatible when floats are used.
    /// This further means that, by default, Wasmtime is unable to load native
    /// code when compiled to the `x86_64-unknown-none` target. The purpose of
    /// this option is to explicitly allow loading code and bypass this check.
    ///
    /// Setting this configuration option to `true` indicates that either:
    /// (a) the Rust target is compiled with the hard-float ABI manually via
    /// `-Zbuild-std` and a custom target JSON configuration, or (b) sufficient
    /// x86 features have been enabled in the compiler such that float libcalls
    /// will not be used in Wasmtime. For (a) there is no way in Rust at this
    /// time to detect whether a hard-float or soft-float ABI is in use on
    /// stable Rust, so this manual opt-in is required. For (b) the only
    /// instance where Wasmtime passes a floating-point value in a register
    /// between the host and compiled wasm code is with libcalls.
    ///
    /// Float-based libcalls are only used when the compilation target for a
    /// wasm module has insufficient target features enabled for native
    /// support. For example SSE4.1 is required for the `f32.ceil` WebAssembly
    /// instruction to be compiled to a native instruction. If SSE4.1 is not
    /// enabled then `f32.ceil` is translated to a "libcall" which is
    /// implemented on the host. Float-based libcalls can be avoided with
    /// sufficient target features enabled, for example:
    ///
    /// * `self.cranelift_flag_enable("has_sse3")`
    /// * `self.cranelift_flag_enable("has_ssse3")`
    /// * `self.cranelift_flag_enable("has_sse41")`
    /// * `self.cranelift_flag_enable("has_sse42")`
    /// * `self.cranelift_flag_enable("has_fma")`
    ///
    /// Note that when these features are enabled Wasmtime will perform a
    /// runtime check to determine that the host actually has the feature
    /// present.
    ///
    /// For some more discussion see [#11506].
    ///
    /// [#11506]: https://github.com/bytecodealliance/wasmtime/issues/11506
    ///
    /// # Safety
    ///
    /// This method is not safe because it cannot be detected in Rust right now
    /// whether the host is compiled with a soft or hard float ABI. Additionally
    /// if the host is compiled with a soft float ABI disabling this check does
    /// not ensure that the wasm module in question has zero usage of floats
    /// in the boundary to the host.
    ///
    /// Safely using this method requires one of:
    ///
    /// * The host target is compiled to use hardware floats.
    /// * Wasm modules loaded are compiled with enough x86 Cranelift features
    ///   enabled to avoid float-related hostcalls.
    pub unsafe fn x86_float_abi_ok(&mut self, enable: bool) -> &mut Self {
        self.x86_float_abi_ok = Some(enable);
        self
    }

    /// Enable or disable the ability to create a
    /// [`SharedMemory`](crate::SharedMemory).
    ///
    /// The WebAssembly threads proposal, configured by [`Config::wasm_threads`]
    /// is on-by-default but there are enough deficiencies in Wasmtime's
    /// implementation and API integration that creation of a shared memory is
    /// disabled by default. This cofiguration knob can be used to enable this.
    ///
    /// When enabling this method be aware that wasm threads are, at this time,
    /// a [tier 2
    /// feature](https://docs.wasmtime.dev/stability-tiers.html#tier-2) in
    /// Wasmtime meaning that it will not receive security updates or fixes to
    /// historical releases. Additionally security CVEs will not be issued for
    /// bugs in the implementation.
    ///
    /// This option is `false` by default.
    pub fn shared_memory(&mut self, enable: bool) -> &mut Self {
        self.shared_memory = enable;
        self
    }

    /// Specifies whether support for concurrent execution of WebAssembly is
    /// supported within this store.
    ///
    /// This configuration option affects whether runtime data structures are
    /// initialized within a `Store` on creation to support concurrent execution
    /// of WebAssembly guests. This is primarily applicable to the
    /// [`Config::wasm_component_model_async`] configuration which is the first
    /// time Wasmtime has supported concurrent execution of guests. This
    /// configuration option, for example, enables usage of
    /// [`Store::run_concurrent`], [`Func::call_concurrent`], [`StreamReader`],
    /// etc.
    ///
    /// This configuration option can be manually disabled to avoid initializing
    /// data structures in the [`Store`] related to concurrent execution. When
    /// this option is disabled then APIs related to concurrency will all fail
    /// with a panic. For example [`Store::run_concurrent`] will panic, creating
    /// a [`StreamReader`] will panic, etc.
    ///
    /// The value of this option additionally affects whether a [`Config`] is
    /// valid and the default set of enabled WebAssembly features. If this
    /// option is disabled then component-model features related to concurrency
    /// will all be disabled. If this option is enabled, then the options will
    /// retain their normal defaults. It is not valid to create a [`Config`]
    /// with component-model-async explicitly enabled and this option explicitly
    /// disabled, however.
    ///
    /// This option defaults to `true`.
    ///
    /// [`Store`]: crate::Store
    /// [`Store::run_concurrent`]: crate::Store::run_concurrent
    /// [`Func::call_concurrent`]: crate::component::Func::call_concurrent
    /// [`StreamReader`]: crate::component::StreamReader
    pub fn concurrency_support(&mut self, enable: bool) -> &mut Self {
        self.tunables.concurrency_support = Some(enable);
        self
    }
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("Config");

        // Not every flag in WasmFeatures can be enabled as part of creating
        // a Config. This impl gives a complete picture of all WasmFeatures
        // enabled, and doesn't require maintenance by hand (which has become out
        // of date in the past), at the cost of possible confusion for why
        // a flag in this set doesn't have a Config setter.
        let features = self.features();
        for flag in WasmFeatures::FLAGS.iter() {
            f.field(
                &format!("wasm_{}", flag.name().to_lowercase()),
                &features.contains(*flag.value()),
            );
        }

        f.field("parallel_compilation", &self.parallel_compilation);
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        {
            f.field("compiler_config", &self.compiler_config);
        }

        self.tunables.format(&mut f);
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

    /// A low-latency baseline compiler for WebAssembly.
    /// For more details regarding ISA support and Wasm proposals support
    /// see <https://docs.wasmtime.dev/stability-tiers.html#current-tier-status>
    Winch,
}

#[cfg(any(feature = "winch", feature = "cranelift"))]
impl Strategy {
    fn not_auto(&self) -> Option<Strategy> {
        match self {
            Strategy::Auto => {
                if cfg!(feature = "cranelift") {
                    Some(Strategy::Cranelift)
                } else if cfg!(feature = "winch") {
                    Some(Strategy::Winch)
                } else {
                    None
                }
            }
            other => Some(*other),
        }
    }
}

/// Possible garbage collector implementations for Wasm.
///
/// This is used as an argument to the [`Config::collector`] method.
///
/// The properties of Wasmtime's available collectors are summarized in the
/// following table:
///
/// | Collector                   | Collects Garbage[^1] | Latency[^2] | Throughput[^3] | Allocation Speed[^4] | Heap Utilization[^5] |
/// |-----------------------------|----------------------|-------------|----------------|----------------------|----------------------|
/// | `DeferredReferenceCounting` | Yes, but not cycles  | 🙂         | 🙁             | 😐                   | 😐                  |
/// | `Null`                      | No                   | 🙂         | 🙂             | 🙂                   | 🙂                  |
///
/// [^1]: Whether or not the collector is capable of collecting garbage and cyclic garbage.
///
/// [^2]: How long the Wasm program is paused during garbage
///       collections. Shorter is better. In general, better latency implies
///       worse throughput and vice versa.
///
/// [^3]: How fast the Wasm program runs when using this collector. Roughly
///       equivalent to the number of Wasm instructions executed per
///       second. Faster is better. In general, better throughput implies worse
///       latency and vice versa.
///
/// [^4]: How fast can individual objects be allocated?
///
/// [^5]: How many objects can the collector fit into N bytes of memory? That
///       is, how much space for bookkeeping and metadata does this collector
///       require? Less space taken up by metadata means more space for
///       additional objects. Reference counts are larger than mark bits and
///       free lists are larger than bump pointers, for example.
#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum Collector {
    /// An indicator that the garbage collector should be automatically
    /// selected.
    ///
    /// This is generally what you want for most projects and indicates that the
    /// `wasmtime` crate itself should make the decision about what the best
    /// collector for a wasm module is.
    ///
    /// Currently this always defaults to the deferred reference-counting
    /// collector, but the default value may change over time.
    Auto,

    /// The deferred reference-counting collector.
    ///
    /// A reference-counting collector, generally trading improved latency for
    /// worsened throughput. However, to avoid the largest overheads of
    /// reference counting, it avoids manipulating reference counts for Wasm
    /// objects on the stack. Instead, it will hold a reference count for an
    /// over-approximation of all objects that are currently on the stack, trace
    /// the stack during collection to find the precise set of on-stack roots,
    /// and decrement the reference count of any object that was in the
    /// over-approximation but not the precise set. This improves throughput,
    /// compared to "pure" reference counting, by performing many fewer
    /// refcount-increment and -decrement operations. The cost is the increased
    /// latency associated with tracing the stack.
    ///
    /// This collector cannot currently collect cycles; they will leak until the
    /// GC heap's store is dropped.
    DeferredReferenceCounting,

    /// The null collector.
    ///
    /// This collector does not actually collect any garbage. It simply
    /// allocates objects until it runs out of memory, at which point further
    /// objects allocation attempts will trap.
    ///
    /// This collector is useful for incredibly short-running Wasm instances
    /// where additionally you would rather halt an over-allocating Wasm program
    /// than spend time collecting its garbage to allow it to keep running. It
    /// is also useful for measuring the overheads associated with other
    /// collectors, as this collector imposes as close to zero throughput and
    /// latency overhead as possible.
    Null,
}

impl Default for Collector {
    fn default() -> Collector {
        Collector::Auto
    }
}

#[cfg(feature = "gc")]
impl Collector {
    fn not_auto(&self) -> Option<Collector> {
        match self {
            Collector::Auto => {
                if cfg!(feature = "gc-drc") {
                    Some(Collector::DeferredReferenceCounting)
                } else if cfg!(feature = "gc-null") {
                    Some(Collector::Null)
                } else {
                    None
                }
            }
            other => Some(*other),
        }
    }

    fn try_not_auto(&self) -> Result<Self> {
        match self.not_auto() {
            #[cfg(feature = "gc-drc")]
            Some(c @ Collector::DeferredReferenceCounting) => Ok(c),
            #[cfg(not(feature = "gc-drc"))]
            Some(Collector::DeferredReferenceCounting) => bail!(
                "cannot create an engine using the deferred reference-counting \
                 collector because the `gc-drc` feature was not enabled at \
                 compile time",
            ),

            #[cfg(feature = "gc-null")]
            Some(c @ Collector::Null) => Ok(c),
            #[cfg(not(feature = "gc-null"))]
            Some(Collector::Null) => bail!(
                "cannot create an engine using the null collector because \
                 the `gc-null` feature was not enabled at compile time",
            ),

            Some(Collector::Auto) => unreachable!(),

            None => bail!(
                "cannot create an engine with GC support when none of the \
                 collectors are available; enable one of the following \
                 features: `gc-drc`, `gc-null`",
            ),
        }
    }
}

/// Possible optimization levels for the Cranelift codegen backend.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

/// Possible register allocator algorithms for the Cranelift codegen backend.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RegallocAlgorithm {
    /// Generates the fastest possible code, but may take longer.
    ///
    /// This algorithm performs "backtracking", which means that it may
    /// undo its earlier work and retry as it discovers conflicts. This
    /// results in better register utilization, producing fewer spills
    /// and moves, but can cause super-linear compile runtime.
    Backtracking,
    /// Generates acceptable code very quickly.
    ///
    /// This algorithm performs a single pass through the code,
    /// guaranteed to work in linear time.  (Note that the rest of
    /// Cranelift is not necessarily guaranteed to run in linear time,
    /// however.) It cannot undo earlier decisions, however, and it
    /// cannot foresee constraints or issues that may occur further
    /// ahead in the code, so the code may have more spills and moves as
    /// a result.
    ///
    /// > **Note**: This algorithm is not yet production-ready and has
    /// > historically had known problems. It is not recommended to enable this
    /// > algorithm for security-sensitive applications and the Wasmtime project
    /// > does not consider this configuration option for issuing security
    /// > advisories at this time.
    SinglePass,
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

    /// Support for profiling Pulley, Wasmtime's interpreter. Note that enabling
    /// this at runtime requires enabling the `profile-pulley` Cargo feature at
    /// compile time.
    Pulley,
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

/// Describe the tri-state configuration of keys such as MPK or PAGEMAP_SCAN.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Enabled {
    /// Enable this feature if it's detected on the host system, otherwise leave
    /// it disabled.
    Auto,
    /// Enable this feature and fail configuration if the feature is not
    /// detected on the host system.
    Yes,
    /// Do not enable this feature, even if the host system supports it.
    No,
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
/// state. For example a pooling allocator can be configured with:
///
/// * [`Config::memory_init_cow`] disabled
/// * [`Config::memory_guard_size`] disabled
/// * [`Config::memory_reservation`] shrunk to minimal size
/// * [`PoolingAllocationConfig::table_keep_resident`] sufficiently large
/// * [`PoolingAllocationConfig::linear_memory_keep_resident`] sufficiently large
///
/// With all these options in place no virtual memory tricks are used at all and
/// everything is manually managed by Wasmtime (for example resetting memory is
/// a `memset(0)`). This is not as fast in a single-threaded scenario but can
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
/// Wasmtime are, by default 4G address space reservations with a small guard
/// region both before and after the linear memory. Memories in the pooling
/// allocator are contiguous which means that we only need a guard after linear
/// memory because the previous linear memory's slot post-guard is our own
/// pre-guard. This means that, by default, the pooling allocator uses roughly
/// 4G of virtual memory per WebAssembly linear memory slot. 4G of virtual
/// memory is 32 bits of a 64-bit address. Many 64-bit systems can only
/// actually use 48-bit addresses by default (although this can be extended on
/// architectures nowadays too), and of those 48 bits one of them is reserved
/// to indicate kernel-vs-userspace. This leaves 47-32=15 bits left,
/// meaning you can only have at most 32k slots of linear memories on many
/// systems by default. This is a relatively small number and shows how the
/// pooling allocator can quickly exhaust all of virtual memory.
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
    config: crate::runtime::vm::PoolingInstanceAllocatorConfig,
}

#[cfg(feature = "pooling-allocator")]
impl PoolingAllocationConfig {
    /// Returns a new configuration builder with all default settings
    /// configured.
    pub fn new() -> PoolingAllocationConfig {
        PoolingAllocationConfig::default()
    }

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

    /// The target number of decommits to do per batch.
    ///
    /// This is not precise, as we can queue up decommits at times when we
    /// aren't prepared to immediately flush them, and so we may go over this
    /// target size occasionally.
    ///
    /// A batch size of one effectively disables batching.
    ///
    /// Defaults to `1`.
    pub fn decommit_batch_size(&mut self, batch_size: usize) -> &mut Self {
        self.config.decommit_batch_size = batch_size;
        self
    }

    /// How much memory, in bytes, to keep resident for async stacks allocated
    /// with the pooling allocator.
    ///
    /// When [`Config::async_stack_zeroing`] is enabled then Wasmtime will reset
    /// the contents of async stacks back to zero upon deallocation. This option
    /// can be used to perform the zeroing operation with `memset` up to a
    /// certain threshold of bytes instead of using system calls to reset the
    /// stack to zero.
    ///
    /// Note that when using this option the memory with async stacks will
    /// never be decommitted.
    #[cfg(feature = "async")]
    pub fn async_stack_keep_resident(&mut self, size: usize) -> &mut Self {
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
    /// (default is unlimited).
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
    /// transitively contain (default is unlimited).
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
    /// contain (default is unlimited).
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
    /// `20000`).
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
    pub fn table_elements(&mut self, elements: usize) -> &mut Self {
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

    /// The maximum byte size that any WebAssembly linear memory may grow to.
    ///
    /// This option defaults to 4 GiB meaning that for 32-bit linear memories
    /// there is no restrictions. 64-bit linear memories will not be allowed to
    /// grow beyond 4 GiB by default.
    ///
    /// If a memory's minimum size is greater than this value, the module will
    /// fail to instantiate.
    ///
    /// If a memory's maximum size is unbounded or greater than this value, the
    /// maximum will be `max_memory_size` for the purpose of any `memory.grow`
    /// instruction.
    ///
    /// This value is used to control the maximum accessible space for each
    /// linear memory of a core instance. This can be thought of as a simple
    /// mechanism like [`Store::limiter`](crate::Store::limiter) to limit memory
    /// at runtime. This value can also affect striping/coloring behavior when
    /// used in conjunction with
    /// [`memory_protection_keys`](PoolingAllocationConfig::memory_protection_keys).
    ///
    /// The virtual memory reservation size of each linear memory is controlled
    /// by the [`Config::memory_reservation`] setting and this method's
    /// configuration cannot exceed [`Config::memory_reservation`].
    pub fn max_memory_size(&mut self, bytes: usize) -> &mut Self {
        self.config.limits.max_memory_size = bytes;
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
    /// Leveraging MPK requires configuring a smaller-than-default
    /// [`max_memory_size`](PoolingAllocationConfig::max_memory_size) to enable
    /// this coloring/striping behavior. For example embeddings might want to
    /// reduce the default 4G allowance to 128M.
    ///
    /// MPK is only available on Linux (called `pku` there) and recent x86
    /// systems; we check for MPK support at runtime by examining the `CPUID`
    /// register. This configuration setting can be in three states:
    ///
    /// - `auto`: if MPK support is available the guard regions are removed; if
    ///   not, the guard regions remain
    /// - `yes`: use MPK to eliminate guard regions; fail if MPK is not
    ///   supported
    /// - `no`: never use MPK
    ///
    /// By default this value is `no`, but may become `auto` in future
    /// releases.
    ///
    /// __WARNING__: this configuration options is still experimental--use at
    /// your own risk! MPK uses kernel and CPU features to protect memory
    /// regions; you may observe segmentation faults if anything is
    /// misconfigured.
    #[cfg(feature = "memory-protection-keys")]
    pub fn memory_protection_keys(&mut self, enable: Enabled) -> &mut Self {
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
    #[cfg(feature = "memory-protection-keys")]
    pub fn max_memory_protection_keys(&mut self, max: usize) -> &mut Self {
        self.config.max_memory_protection_keys = max;
        self
    }

    /// Check if memory protection keys (MPK) are available on the current host.
    ///
    /// This is a convenience method for determining MPK availability using the
    /// same method that [`Enabled::Auto`] does. See
    /// [`PoolingAllocationConfig::memory_protection_keys`] for more
    /// information.
    #[cfg(feature = "memory-protection-keys")]
    pub fn are_memory_protection_keys_available() -> bool {
        crate::runtime::vm::mpk::is_supported()
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
    pub fn total_gc_heaps(&mut self, count: u32) -> &mut Self {
        self.config.limits.total_gc_heaps = count;
        self
    }

    /// Configures whether the Linux-specific [`PAGEMAP_SCAN` ioctl][ioctl] is
    /// used to help reset linear memory.
    ///
    /// When [`Self::linear_memory_keep_resident`] or
    /// [`Self::table_keep_resident`] options are configured to nonzero values
    /// the default behavior is to `memset` the lowest addresses of a table or
    /// memory back to their original contents. With the `PAGEMAP_SCAN` ioctl on
    /// Linux this can be done to more intelligently scan for resident pages in
    /// the region and only reset those pages back to their original contents
    /// with `memset` rather than assuming the low addresses are all resident.
    ///
    /// This ioctl has the potential to provide a number of performance benefits
    /// in high-reuse and high concurrency scenarios. Notably this enables
    /// Wasmtime to scan the entire region of WebAssembly linear memory and
    /// manually reset memory back to its original contents, up to
    /// [`Self::linear_memory_keep_resident`] bytes, possibly skipping an
    /// `madvise` entirely. This can be more efficient by avoiding removing
    /// pages from the address space entirely and additionally ensuring that
    /// future use of the linear memory doesn't incur page faults as the pages
    /// remain resident.
    ///
    /// At this time this configuration option is still being evaluated as to
    /// how appropriate it is for all use cases. It currently defaults to
    /// `no` or disabled but may change to `auto`, enable if supported, in the
    /// future. This option is only supported on Linux and requires a kernel
    /// version of 6.7 or higher.
    ///
    /// [ioctl]: https://www.man7.org/linux/man-pages/man2/PAGEMAP_SCAN.2const.html
    pub fn pagemap_scan(&mut self, enable: Enabled) -> &mut Self {
        self.config.pagemap_scan = enable;
        self
    }

    /// Tests whether [`Self::pagemap_scan`] is available or not on the host
    /// system.
    pub fn is_pagemap_scan_available() -> bool {
        crate::runtime::vm::PoolingInstanceAllocatorConfig::is_pagemap_scan_available()
    }
}

#[cfg(feature = "std")]
fn detect_host_feature(feature: &str) -> Option<bool> {
    #[cfg(target_arch = "aarch64")]
    {
        return match feature {
            "lse" => Some(std::arch::is_aarch64_feature_detected!("lse")),
            "paca" => Some(std::arch::is_aarch64_feature_detected!("paca")),
            "fp16" => Some(std::arch::is_aarch64_feature_detected!("fp16")),

            _ => None,
        };
    }

    // `is_s390x_feature_detected` is nightly only for now, so use the
    // STORE FACILITY LIST EXTENDED instruction as a temporary measure.
    #[cfg(target_arch = "s390x")]
    {
        let mut facility_list: [u64; 4] = [0; 4];
        unsafe {
            core::arch::asm!(
                "stfle 0({})",
                in(reg_addr) facility_list.as_mut_ptr() ,
                inout("r0") facility_list.len() as u64 - 1 => _,
                options(nostack)
            );
        }
        let get_facility_bit = |n: usize| {
            // NOTE: bits are numbered from the left.
            facility_list[n / 64] & (1 << (63 - (n % 64))) != 0
        };

        return match feature {
            "mie3" => Some(get_facility_bit(61)),
            "mie4" => Some(get_facility_bit(84)),
            "vxrs_ext2" => Some(get_facility_bit(148)),
            "vxrs_ext3" => Some(get_facility_bit(198)),

            _ => None,
        };
    }

    #[cfg(target_arch = "riscv64")]
    {
        return match feature {
            // due to `is_riscv64_feature_detected` is not stable.
            // we cannot use it. For now lie and say all features are always
            // found to keep tests working.
            _ => Some(true),
        };
    }

    #[cfg(target_arch = "x86_64")]
    {
        return match feature {
            "cmpxchg16b" => Some(std::is_x86_feature_detected!("cmpxchg16b")),
            "sse3" => Some(std::is_x86_feature_detected!("sse3")),
            "ssse3" => Some(std::is_x86_feature_detected!("ssse3")),
            "sse4.1" => Some(std::is_x86_feature_detected!("sse4.1")),
            "sse4.2" => Some(std::is_x86_feature_detected!("sse4.2")),
            "popcnt" => Some(std::is_x86_feature_detected!("popcnt")),
            "avx" => Some(std::is_x86_feature_detected!("avx")),
            "avx2" => Some(std::is_x86_feature_detected!("avx2")),
            "fma" => Some(std::is_x86_feature_detected!("fma")),
            "bmi1" => Some(std::is_x86_feature_detected!("bmi1")),
            "bmi2" => Some(std::is_x86_feature_detected!("bmi2")),
            "avx512bitalg" => Some(std::is_x86_feature_detected!("avx512bitalg")),
            "avx512dq" => Some(std::is_x86_feature_detected!("avx512dq")),
            "avx512f" => Some(std::is_x86_feature_detected!("avx512f")),
            "avx512vl" => Some(std::is_x86_feature_detected!("avx512vl")),
            "avx512vbmi" => Some(std::is_x86_feature_detected!("avx512vbmi")),
            "lzcnt" => Some(std::is_x86_feature_detected!("lzcnt")),

            _ => None,
        };
    }

    #[allow(
        unreachable_code,
        reason = "reachable or not depending on if a target above matches"
    )]
    {
        let _ = feature;
        return None;
    }
}
