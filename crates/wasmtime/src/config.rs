use crate::memory::MemoryCreator;
use crate::trampoline::MemoryCreatorProxy;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::cmp;
use std::fmt;
#[cfg(feature = "cache")]
use std::path::Path;
use std::sync::Arc;
use wasmparser::WasmFeatures;
#[cfg(feature = "cache")]
use wasmtime_cache::CacheConfig;
use wasmtime_environ::{CompilerBuilder, Tunables};
use wasmtime_jit::{JitDumpAgent, NullProfilerAgent, ProfilingAgent, VTuneAgent};
use wasmtime_runtime::{InstanceAllocator, OnDemandInstanceAllocator, RuntimeMemoryCreator};

#[cfg(feature = "pooling-allocator")]
pub use wasmtime_runtime::{InstanceLimits, PoolingAllocationStrategy};

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
    Pooling {
        /// The allocation strategy to use.
        strategy: PoolingAllocationStrategy,
        /// The instance limits to use.
        instance_limits: InstanceLimits,
    },
}

impl InstanceAllocationStrategy {
    /// The default pooling instance allocation strategy.
    #[cfg(feature = "pooling-allocator")]
    pub fn pooling() -> Self {
        Self::Pooling {
            strategy: PoolingAllocationStrategy::default(),
            instance_limits: InstanceLimits::default(),
        }
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

/// Global configuration options used to create an [`Engine`](crate::Engine)
/// and customize its behavior.
///
/// This structure exposed a builder-like interface and is primarily consumed by
/// [`Engine::new()`](crate::Engine::new)
pub struct Config {
    #[cfg(compiler)]
    pub(crate) compiler: Box<dyn CompilerBuilder>,
    pub(crate) tunables: Tunables,
    #[cfg(feature = "cache")]
    pub(crate) cache_config: CacheConfig,
    pub(crate) profiler: Arc<dyn ProfilingAgent>,
    pub(crate) mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>,
    pub(crate) allocation_strategy: InstanceAllocationStrategy,
    pub(crate) max_wasm_stack: usize,
    pub(crate) features: WasmFeatures,
    pub(crate) wasm_backtrace_details_env_used: bool,
    #[cfg(feature = "async")]
    pub(crate) async_stack_size: usize,
    pub(crate) async_support: bool,
    pub(crate) module_version: ModuleVersionStrategy,
    pub(crate) parallel_compilation: bool,
    pub(crate) memory_init_cow: bool,
    pub(crate) memory_guaranteed_dense_image_size: u64,
    pub(crate) force_memory_init_memfd: bool,
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        let mut ret = Self {
            tunables: Tunables::default(),
            #[cfg(compiler)]
            compiler: compiler_builder(Strategy::Auto).unwrap(),
            #[cfg(feature = "cache")]
            cache_config: CacheConfig::new_cache_disabled(),
            profiler: Arc::new(NullProfilerAgent),
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
            wasm_backtrace_details_env_used: false,
            features: WasmFeatures::default(),
            #[cfg(feature = "async")]
            async_stack_size: 2 << 20,
            async_support: false,
            module_version: ModuleVersionStrategy::default(),
            parallel_compilation: true,
            memory_init_cow: true,
            memory_guaranteed_dense_image_size: 16 << 20,
            force_memory_init_memfd: false,
        };
        #[cfg(compiler)]
        {
            ret.cranelift_debug_verifier(false);
            ret.cranelift_opt_level(OptLevel::Speed);
        }
        #[cfg(feature = "wasm-backtrace")]
        ret.wasm_reference_types(true);
        ret.features.reference_types = cfg!(feature = "wasm-backtrace");
        ret.wasm_multi_value(true);
        ret.wasm_bulk_memory(true);
        ret.wasm_simd(true);
        ret.wasm_backtrace_details(WasmBacktraceDetails::Environment);
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
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn target(&mut self, target: &str) -> Result<&mut Self> {
        use std::str::FromStr;
        self.compiler
            .target(target_lexicon::Triple::from_str(target).map_err(|e| anyhow::anyhow!(e))?)?;

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
    ///   as [`crate::Store::out_of_fuel_async_yield`] When doing so this will
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
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub fn async_support(&mut self, enable: bool) -> &mut Self {
        self.async_support = enable;
        self
    }

    /// Configures whether DWARF debug information will be emitted during
    /// compilation.
    ///
    /// By default this option is `false`.
    pub fn debug_info(&mut self, enable: bool) -> &mut Self {
        self.tunables.generate_native_debuginfo = enable;
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
            WasmBacktraceDetails::Enable => true,
            WasmBacktraceDetails::Disable => false,
            WasmBacktraceDetails::Environment => {
                self.wasm_backtrace_details_env_used = true;
                std::env::var("WASMTIME_BACKTRACE_DETAILS")
                    .map(|s| s == "1")
                    .unwrap_or(false)
            }
        };
        self
    }

    /// Configures whether execution of WebAssembly will "consume fuel" to
    /// either halt or yield execution as desired.
    ///
    /// This can be used to deterministically prevent infinitely-executing
    /// WebAssembly code by instrumenting generated code to consume fuel as it
    /// executes. When fuel runs out the behavior is defined by configuration
    /// within a [`Store`], and by default a trap is raised.
    ///
    /// Note that a [`Store`] starts with no fuel, so if you enable this option
    /// you'll have to be sure to pour some fuel into [`Store`] before
    /// executing some code.
    ///
    /// By default this option is `false`.
    ///
    /// [`Store`]: crate::Store
    pub fn consume_fuel(&mut self, enable: bool) -> &mut Self {
        self.tunables.consume_fuel = enable;
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
    /// An initial deadline can be set before executing code by
    /// calling
    /// [`Store::set_epoch_deadline`](crate::Store::set_epoch_deadline).
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
        self.tunables.epoch_interruption = enable;
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
    /// When the `async` feature is enabled, this value cannot exceed the
    /// `async_stack_size` option. Be careful not to set this value too close
    /// to `async_stack_size` as doing so may limit how much stack space
    /// is available for host functions. Unlike wasm functions that trap
    /// on stack overflow, a host function that overflows the stack will
    /// abort the process.
    ///
    /// By default this option is 512 KiB.
    pub fn max_wasm_stack(&mut self, size: usize) -> Result<&mut Self> {
        #[cfg(feature = "async")]
        if size > self.async_stack_size {
            bail!("wasm stack size cannot exceed the async stack size");
        }

        if size == 0 {
            bail!("wasm stack size cannot be zero");
        }

        self.max_wasm_stack = size;
        Ok(self)
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
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub fn async_stack_size(&mut self, size: usize) -> Result<&mut Self> {
        if size < self.max_wasm_stack {
            bail!("async stack size cannot be less than the maximum wasm stack size");
        }
        self.async_stack_size = size;
        Ok(self)
    }

    /// Configures whether the WebAssembly threads proposal will be enabled for
    /// compilation.
    ///
    /// The [WebAssembly threads proposal][threads] is not currently fully
    /// standardized and is undergoing development. Additionally the support in
    /// wasmtime itself is still being worked on. Support for this feature can
    /// be enabled through this method for appropriate wasm modules.
    ///
    /// This feature gates items such as shared memories and atomic
    /// instructions. Note that enabling the threads feature will
    /// also enable the bulk memory feature.
    ///
    /// This is `false` by default.
    ///
    /// > **Note**: Wasmtime does not implement everything for the wasm threads
    /// > spec at this time, so bugs, panics, and possibly segfaults should be
    /// > expected. This should not be enabled in a production setting right
    /// > now.
    ///
    /// [threads]: https://github.com/webassembly/threads
    pub fn wasm_threads(&mut self, enable: bool) -> &mut Self {
        self.features.threads = enable;
        // The threads proposal depends on the bulk memory proposal
        if enable {
            self.wasm_bulk_memory(true);
        }
        self
    }

    /// Configures whether the [WebAssembly reference types proposal][proposal]
    /// will be enabled for compilation.
    ///
    /// This feature gates items such as the `externref` and `funcref` types as
    /// well as allowing a module to define multiple tables.
    ///
    /// Note that enabling the reference types feature will also enable the bulk
    /// memory feature.
    ///
    /// This feature is `true` by default. If the `wasm-backtrace` feature is
    /// disabled at compile time, however, then this is `false` by default and
    /// it cannot be turned on since GC currently requires backtraces to work.
    /// Note that the `wasm-backtrace` feature is on by default, however.
    ///
    /// [proposal]: https://github.com/webassembly/reference-types
    #[cfg(feature = "wasm-backtrace")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "wasm-backtrace")))]
    pub fn wasm_reference_types(&mut self, enable: bool) -> &mut Self {
        self.features.reference_types = enable;

        #[cfg(compiler)]
        {
            self.compiler
                .set("enable_safepoints", if enable { "true" } else { "false" })
                .unwrap();
        }

        // The reference types proposal depends on the bulk memory proposal.
        if enable {
            self.wasm_bulk_memory(true);
        }

        self
    }

    /// Configures whether the WebAssembly SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly SIMD proposal][proposal]. This feature gates items such
    /// as the `v128` type and all of its operators being in a module. Note that
    /// this does not enable the [relaxed simd proposal] as that is not
    /// implemented in Wasmtime at this time.
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
        self.features.simd = enable;
        #[cfg(compiler)]
        {
            let val = if enable { "true" } else { "false" };
            self.compiler
                .set("enable_simd", val)
                .expect("should be valid flag");
        }
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
    /// [proposal]: https://github.com/webassembly/bulk-memory-operations
    pub fn wasm_bulk_memory(&mut self, enable: bool) -> &mut Self {
        self.features.bulk_memory = enable;
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
        self.features.multi_value = enable;
        self
    }

    /// Configures whether the WebAssembly multi-memory [proposal] will
    /// be enabled for compilation.
    ///
    /// This feature gates modules having more than one linear memory
    /// declaration or import.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/multi-memory
    pub fn wasm_multi_memory(&mut self, enable: bool) -> &mut Self {
        self.features.multi_memory = enable;
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
        self.features.memory64 = enable;
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
    /// # Errors
    ///
    /// Some compilation strategies require compile-time options of `wasmtime`
    /// itself to be set, but if they're not set and the strategy is specified
    /// here then an error will be returned.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn strategy(&mut self, strategy: Strategy) -> Result<&mut Self> {
        self.compiler = compiler_builder(strategy)?;
        Ok(self)
    }

    /// Creates a default profiler based on the profiling strategy chosen.
    ///
    /// Profiler creation calls the type's default initializer where the purpose is
    /// really just to put in place the type used for profiling.
    pub fn profiler(&mut self, profile: ProfilingStrategy) -> Result<&mut Self> {
        self.profiler = match profile {
            ProfilingStrategy::JitDump => Arc::new(JitDumpAgent::new()?) as Arc<dyn ProfilingAgent>,
            ProfilingStrategy::VTune => Arc::new(VTuneAgent::new()?) as Arc<dyn ProfilingAgent>,
            ProfilingStrategy::None => Arc::new(NullProfilerAgent),
        };
        Ok(self)
    }

    /// Configures whether the debug verifier of Cranelift is enabled or not.
    ///
    /// When Cranelift is used as a code generation backend this will configure
    /// it to have the `enable_verifier` flag which will enable a number of debug
    /// checks inside of Cranelift. This is largely only useful for the
    /// developers of wasmtime itself.
    ///
    /// The default value for this is `false`
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn cranelift_debug_verifier(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler
            .set("enable_verifier", val)
            .expect("should be valid flag");
        self
    }

    /// Configures the Cranelift code generator optimization level.
    ///
    /// When the Cranelift code generator is used you can configure the
    /// optimization level used for generated code in a few various ways. For
    /// more information see the documentation of [`OptLevel`].
    ///
    /// The default value for this is `OptLevel::None`.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn cranelift_opt_level(&mut self, level: OptLevel) -> &mut Self {
        let val = match level {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        };
        self.compiler
            .set("opt_level", val)
            .expect("should be valid flag");
        self
    }

    /// Configures whether Cranelift should perform a NaN-canonicalization pass.
    ///
    /// When Cranelift is used as a code generation backend this will configure
    /// it to replace NaNs with a single canonical value. This is useful for users
    /// requiring entirely deterministic WebAssembly computation.
    /// This is not required by the WebAssembly spec, so it is not enabled by default.
    ///
    /// The default value for this is `false`
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub fn cranelift_nan_canonicalization(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.compiler
            .set("enable_nan_canonicalization", val)
            .expect("should be valid flag");
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
    /// This method can fail if the flag's name does not exist.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub unsafe fn cranelift_flag_enable(&mut self, flag: &str) -> Result<&mut Self> {
        self.compiler.enable(flag)?;
        Ok(self)
    }

    /// Allows settings another Cranelift flag defined by a flag name and value. This allows
    /// fine-tuning of Cranelift settings.
    ///
    /// Since Cranelift flags may be unstable, this method should not be considered to be stable
    /// either; other `Config` functions should be preferred for stability.
    ///
    /// Note that this is marked as unsafe, because setting the wrong flag might break invariants,
    /// resulting in execution hazards.
    ///
    /// # Errors
    ///
    /// This method can fail if the flag's name does not exist, or the value is not appropriate for
    /// the flag type.
    #[cfg(compiler)]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cranelift")))] // see build.rs
    pub unsafe fn cranelift_flag_set(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        self.compiler.set(name, value)?;
        Ok(self)
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
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cache")))]
    pub fn cache_config_load(&mut self, path: impl AsRef<Path>) -> Result<&mut Self> {
        self.cache_config = CacheConfig::from_file(Some(path.as_ref()))?;
        Ok(self)
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
    #[cfg_attr(nightlydoc, doc(cfg(feature = "cache")))]
    pub fn cache_config_load_default(&mut self) -> Result<&mut Self> {
        self.cache_config = CacheConfig::from_file(None)?;
        Ok(self)
    }

    /// Sets a custom memory creator.
    ///
    /// Custom memory creators are used when creating host `Memory` objects or when
    /// creating instance linear memories for the on-demand instance allocation strategy.
    pub fn with_host_memory(&mut self, mem_creator: Arc<dyn MemoryCreator>) -> &mut Self {
        self.mem_creator = Some(Arc::new(MemoryCreatorProxy(mem_creator)));
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
    pub fn static_memory_maximum_size(&mut self, max_size: u64) -> &mut Self {
        let max_pages = max_size / u64::from(wasmtime_environ::WASM_PAGE_SIZE);
        self.tunables.static_memory_bound = max_pages;
        self
    }

    /// Indicates that the "static" style of memory should always be used.
    ///
    /// This configuration option enables selecting the "static" option for all
    /// linear memories created within this `Config`. This means that all
    /// memories will be allocated up-front and will never move. Additionally
    /// this means that all memories are synthetically limited by the
    /// [`Config::static_memory_maximum_size`] option, irregardless of what the
    /// actual maximum size is on the memory's original type.
    ///
    /// For the difference between static and dynamic memories, see the
    /// [`Config::static_memory_maximum_size`].
    pub fn static_memory_forced(&mut self, force: bool) -> &mut Self {
        self.tunables.static_memory_bound_is_maximum = force;
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
    /// For 32-bit memories a 4GB static memory is required to even start
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
    /// ## Static vs Dynamic Guard Size
    ///
    /// Note that for now the static memory guard size must be at least as large
    /// as the dynamic memory guard size, so configuring this property to be
    /// smaller than the dynamic memory guard size will have no effect.
    pub fn static_memory_guard_size(&mut self, guard_size: u64) -> &mut Self {
        let guard_size = round_up_to_pages(guard_size);
        let guard_size = cmp::max(guard_size, self.tunables.dynamic_memory_offset_guard_size);
        self.tunables.static_memory_offset_guard_size = guard_size;
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
    pub fn dynamic_memory_guard_size(&mut self, guard_size: u64) -> &mut Self {
        let guard_size = round_up_to_pages(guard_size);
        self.tunables.dynamic_memory_offset_guard_size = guard_size;
        self.tunables.static_memory_offset_guard_size =
            cmp::max(guard_size, self.tunables.static_memory_offset_guard_size);
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
        self.tunables.dynamic_memory_growth_reserve = round_up_to_pages(reserved);
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
        self.tunables.guard_before_linear_memory = guard;
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

    /// Configure wether wasmtime should compile a module using multiple threads.
    ///
    /// Disabling this will result in a single thread being used to compile the wasm bytecode.
    ///
    /// By default parallel compilation is enabled.
    #[cfg(feature = "parallel-compilation")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "parallel-compilation")))]
    pub fn parallel_compilation(&mut self, parallel: bool) -> &mut Self {
        self.parallel_compilation = parallel;
        self
    }

    /// Configures whether compiled artifacts will contain information to map
    /// native program addresses back to the original wasm module.
    ///
    /// This configuration option is `true` by default and, if enables,
    /// generates the appropriate tables in compiled modules to map from native
    /// address back to wasm source addresses. This is used for displaying wasm
    /// program counters in backtraces as well as generating filenames/line
    /// numbers if so configured as well (and the original wasm module has DWARF
    /// debugging information present).
    pub fn generate_address_map(&mut self, generate: bool) -> &mut Self {
        self.tunables.generate_address_map = generate;
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
    #[cfg(feature = "memory-init-cow")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "memory-init-cow")))]
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
    #[cfg(feature = "memory-init-cow")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "memory-init-cow")))]
    pub fn force_memory_init_memfd(&mut self, enable: bool) -> &mut Self {
        self.force_memory_init_memfd = enable;
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
    #[cfg(feature = "memory-init-cow")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "memory-init-cow")))]
    pub fn memory_guaranteed_dense_image_size(&mut self, size_in_bytes: u64) -> &mut Self {
        self.memory_guaranteed_dense_image_size = size_in_bytes;
        self
    }

    pub(crate) fn build_allocator(&self) -> Result<Box<dyn InstanceAllocator>> {
        #[cfg(feature = "async")]
        let stack_size = self.async_stack_size;

        #[cfg(not(feature = "async"))]
        let stack_size = 0;

        match self.allocation_strategy {
            InstanceAllocationStrategy::OnDemand => Ok(Box::new(OnDemandInstanceAllocator::new(
                self.mem_creator.clone(),
                stack_size,
            ))),
            #[cfg(feature = "pooling-allocator")]
            InstanceAllocationStrategy::Pooling {
                strategy,
                instance_limits,
            } => Ok(Box::new(wasmtime_runtime::PoolingInstanceAllocator::new(
                strategy,
                instance_limits,
                stack_size,
                &self.tunables,
            )?)),
        }
    }
}

#[cfg(compiler)]
fn compiler_builder(strategy: Strategy) -> Result<Box<dyn CompilerBuilder>> {
    let mut builder = match strategy {
        Strategy::Auto | Strategy::Cranelift => wasmtime_cranelift::builder(),
    };
    builder
        .set(
            "unwind_info",
            if cfg!(feature = "wasm-backtrace") {
                "true"
            } else {
                "false"
            },
        )
        .unwrap();
    Ok(builder)
}

fn round_up_to_pages(val: u64) -> u64 {
    let page_size = region::page::size() as u64;
    debug_assert!(page_size.is_power_of_two());
    val.checked_add(page_size - 1)
        .map(|val| val & !(page_size - 1))
        .unwrap_or(u64::max_value() / page_size + 1)
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

impl Clone for Config {
    fn clone(&self) -> Config {
        Config {
            #[cfg(compiler)]
            compiler: self.compiler.clone(),
            tunables: self.tunables.clone(),
            #[cfg(feature = "cache")]
            cache_config: self.cache_config.clone(),
            profiler: self.profiler.clone(),
            features: self.features.clone(),
            mem_creator: self.mem_creator.clone(),
            allocation_strategy: self.allocation_strategy.clone(),
            max_wasm_stack: self.max_wasm_stack,
            wasm_backtrace_details_env_used: self.wasm_backtrace_details_env_used,
            async_support: self.async_support,
            #[cfg(feature = "async")]
            async_stack_size: self.async_stack_size,
            module_version: self.module_version.clone(),
            parallel_compilation: self.parallel_compilation,
            memory_init_cow: self.memory_init_cow,
            memory_guaranteed_dense_image_size: self.memory_guaranteed_dense_image_size,
            force_memory_init_memfd: self.force_memory_init_memfd,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("Config");
        f.field("debug_info", &self.tunables.generate_native_debuginfo)
            .field("parse_wasm_debuginfo", &self.tunables.parse_wasm_debuginfo)
            .field("wasm_threads", &self.features.threads)
            .field("wasm_reference_types", &self.features.reference_types)
            .field("wasm_bulk_memory", &self.features.bulk_memory)
            .field("wasm_simd", &self.features.simd)
            .field("wasm_multi_value", &self.features.multi_value)
            .field(
                "static_memory_maximum_size",
                &(u64::from(self.tunables.static_memory_bound)
                    * u64::from(wasmtime_environ::WASM_PAGE_SIZE)),
            )
            .field(
                "static_memory_guard_size",
                &self.tunables.static_memory_offset_guard_size,
            )
            .field(
                "dynamic_memory_guard_size",
                &self.tunables.dynamic_memory_offset_guard_size,
            )
            .field(
                "guard_before_linear_memory",
                &self.tunables.guard_before_linear_memory,
            )
            .field("parallel_compilation", &self.parallel_compilation);
        #[cfg(compiler)]
        {
            f.field("compiler", &self.compiler);
        }
        f.finish()
    }
}

/// Possible Compilation strategies for a wasm module.
///
/// This is used as an argument to the [`Config::strategy`] method.
#[non_exhaustive]
#[derive(Clone, Debug)]
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
}

/// Possible optimization levels for the Cranelift codegen backend.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
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
#[derive(Debug, Clone, Copy)]
pub enum ProfilingStrategy {
    /// No profiler support.
    None,

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
