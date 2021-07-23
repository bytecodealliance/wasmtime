use crate::memory::MemoryCreator;
use crate::trampoline::MemoryCreatorProxy;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::cmp;
use std::convert::TryFrom;
use std::fmt;
#[cfg(feature = "cache")]
use std::path::Path;
use std::sync::Arc;
use wasmparser::WasmFeatures;
#[cfg(feature = "cache")]
use wasmtime_cache::CacheConfig;
use wasmtime_environ::settings::{self, Configurable, SetError};
use wasmtime_environ::{isa, isa::TargetIsa, Tunables};
use wasmtime_jit::{native, CompilationStrategy, Compiler};
use wasmtime_profiling::{JitDumpAgent, NullProfilerAgent, ProfilingAgent, VTuneAgent};
use wasmtime_runtime::{
    InstanceAllocator, OnDemandInstanceAllocator, PoolingInstanceAllocator, RuntimeMemoryCreator,
};

/// Represents the limits placed on a module for compiling with the pooling instance allocation strategy.
#[derive(Debug, Copy, Clone)]
pub struct ModuleLimits {
    /// The maximum number of imported functions for a module (default is 1000).
    ///
    /// This value controls the capacity of the `VMFunctionImport` table and the
    /// `VMCallerCheckedAnyfunc` table in each instance's `VMContext` structure.
    ///
    /// The allocated size of the `VMFunctionImport` table will be `imported_functions * sizeof(VMFunctionImport)`
    /// for each instance regardless of how many functions an instance imports.
    ///
    /// The allocated size of the `VMCallerCheckedAnyfunc` table will be
    /// `imported_functions * functions * sizeof(VMCallerCheckedAnyfunc)` for each instance regardless of
    /// how many functions are imported and defined by an instance.
    pub imported_functions: u32,

    /// The maximum number of imported tables for a module (default is 0).
    ///
    /// This value controls the capacity of the `VMTableImport` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `imported_tables * sizeof(VMTableImport)` for each
    /// instance regardless of how many tables an instance imports.
    pub imported_tables: u32,

    /// The maximum number of imported linear memories for a module (default is 0).
    ///
    /// This value controls the capacity of the `VMMemoryImport` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `imported_memories * sizeof(VMMemoryImport)` for each
    /// instance regardless of how many memories an instance imports.
    pub imported_memories: u32,

    /// The maximum number of imported globals for a module (default is 0).
    ///
    /// This value controls the capacity of the `VMGlobalImport` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `imported_globals * sizeof(VMGlobalImport)` for each
    /// instance regardless of how many globals an instance imports.
    pub imported_globals: u32,

    /// The maximum number of defined types for a module (default is 100).
    ///
    /// This value controls the capacity of the `VMSharedSignatureIndex` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `types * sizeof(VMSharedSignatureIndex)` for each
    /// instance regardless of how many types are defined by an instance's module.
    pub types: u32,

    /// The maximum number of defined functions for a module (default is 10000).
    ///
    /// This value controls the capacity of the `VMCallerCheckedAnyfunc` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the `VMCallerCheckedAnyfunc` table will be
    /// `imported_functions * functions * sizeof(VMCallerCheckedAnyfunc)` for each instance
    /// regardless of how many functions are imported and defined by an instance.
    pub functions: u32,

    /// The maximum number of defined tables for a module (default is 1).
    ///
    /// This value controls the capacity of the `VMTableDefinition` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `tables * sizeof(VMTableDefinition)` for each
    /// instance regardless of how many tables are defined by an instance's module.
    pub tables: u32,

    /// The maximum number of defined linear memories for a module (default is 1).
    ///
    /// This value controls the capacity of the `VMMemoryDefinition` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `memories * sizeof(VMMemoryDefinition)` for each
    /// instance regardless of how many memories are defined by an instance's module.
    pub memories: u32,

    /// The maximum number of defined globals for a module (default is 10).
    ///
    /// This value controls the capacity of the `VMGlobalDefinition` table in each instance's
    /// `VMContext` structure.
    ///
    /// The allocated size of the table will be `globals * sizeof(VMGlobalDefinition)` for each
    /// instance regardless of how many globals are defined by an instance's module.
    pub globals: u32,

    /// The maximum table elements for any table defined in a module (default is 10000).
    ///
    /// If a table's minimum element limit is greater than this value, the module will
    /// fail to compile.
    ///
    /// If a table's maximum element limit is unbounded or greater than this value,
    /// the maximum will be `table_elements` for the purpose of any `table.grow` instruction.
    ///
    /// This value is used to reserve the maximum space for each supported table; table elements
    /// are pointer-sized in the Wasmtime runtime.  Therefore, the space reserved for each instance
    /// is `tables * table_elements * sizeof::<*const ()>`.
    pub table_elements: u32,

    /// The maximum number of pages for any linear memory defined in a module (default is 160).
    ///
    /// The default of 160 means at most 10 MiB of host memory may be committed for each instance.
    ///
    /// If a memory's minimum page limit is greater than this value, the module will
    /// fail to compile.
    ///
    /// If a memory's maximum page limit is unbounded or greater than this value,
    /// the maximum will be `memory_pages` for the purpose of any `memory.grow` instruction.
    ///
    /// This value is used to control the maximum accessible space for each linear memory of an instance.
    ///
    /// The reservation size of each linear memory is controlled by the
    /// [`static_memory_maximum_size`](Config::static_memory_maximum_size) setting and this value cannot
    /// exceed the configured static memory maximum size.
    pub memory_pages: u32,
}

impl Default for ModuleLimits {
    fn default() -> Self {
        // Use the defaults from the runtime
        let wasmtime_runtime::ModuleLimits {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        } = wasmtime_runtime::ModuleLimits::default();

        Self {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        }
    }
}

// This exists so we can convert between the public Wasmtime API and the runtime representation
// without having to export runtime types from the Wasmtime API.
#[doc(hidden)]
impl Into<wasmtime_runtime::ModuleLimits> for ModuleLimits {
    fn into(self) -> wasmtime_runtime::ModuleLimits {
        let Self {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        } = self;

        wasmtime_runtime::ModuleLimits {
            imported_functions,
            imported_tables,
            imported_memories,
            imported_globals,
            types,
            functions,
            tables,
            memories,
            globals,
            table_elements,
            memory_pages,
        }
    }
}

/// Represents the limits placed on instances by the pooling instance allocation strategy.
#[derive(Debug, Copy, Clone)]
pub struct InstanceLimits {
    /// The maximum number of concurrent instances supported (default is 1000).
    ///
    /// This value has a direct impact on the amount of memory allocated by the pooling
    /// instance allocator.
    ///
    /// The pooling instance allocator allocates three memory pools with sizes depending on this value:
    ///
    /// * An instance pool, where each entry in the pool can store the runtime representation
    ///   of an instance, including a maximal `VMContext` structure (see [`ModuleLimits`](ModuleLimits)
    ///   for the various settings that control the size of each instance's `VMContext` structure).
    ///
    /// * A memory pool, where each entry in the pool contains the reserved address space for each
    ///   linear memory supported by an instance.
    ///
    /// * A table pool, where each entry in the pool contains the space needed for each WebAssembly table
    ///   supported by an instance (see `[ModuleLimits::table_elements`] to control the size of each table).
    ///
    /// Additionally, this value will also control the maximum number of execution stacks allowed for
    /// asynchronous execution (one per instance), when enabled.
    ///
    /// The memory pool will reserve a large quantity of host process address space to elide the bounds
    /// checks required for correct WebAssembly memory semantics. Even for 64-bit address spaces, the
    /// address space is limited when dealing with a large number of supported instances.
    ///
    /// For example, on Linux x86_64, the userland address space limit is 128 TiB. That might seem like a lot,
    /// but each linear memory will *reserve* 6 GiB of space by default. Multiply that by the number of linear
    /// memories each instance supports and then by the number of supported instances and it becomes apparent
    /// that address space can be exhausted depending on the number of supported instances.
    pub count: u32,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        let wasmtime_runtime::InstanceLimits { count } =
            wasmtime_runtime::InstanceLimits::default();

        Self { count }
    }
}

// This exists so we can convert between the public Wasmtime API and the runtime representation
// without having to export runtime types from the Wasmtime API.
#[doc(hidden)]
impl Into<wasmtime_runtime::InstanceLimits> for InstanceLimits {
    fn into(self) -> wasmtime_runtime::InstanceLimits {
        let Self { count } = self;

        wasmtime_runtime::InstanceLimits { count }
    }
}

/// The allocation strategy to use for the pooling instance allocation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingAllocationStrategy {
    /// Allocate from the next available instance.
    NextAvailable,
    /// Allocate from a random available instance.
    Random,
}

impl Default for PoolingAllocationStrategy {
    fn default() -> Self {
        match wasmtime_runtime::PoolingAllocationStrategy::default() {
            wasmtime_runtime::PoolingAllocationStrategy::NextAvailable => Self::NextAvailable,
            wasmtime_runtime::PoolingAllocationStrategy::Random => Self::Random,
        }
    }
}

// This exists so we can convert between the public Wasmtime API and the runtime representation
// without having to export runtime types from the Wasmtime API.
#[doc(hidden)]
impl Into<wasmtime_runtime::PoolingAllocationStrategy> for PoolingAllocationStrategy {
    fn into(self) -> wasmtime_runtime::PoolingAllocationStrategy {
        match self {
            Self::NextAvailable => wasmtime_runtime::PoolingAllocationStrategy::NextAvailable,
            Self::Random => wasmtime_runtime::PoolingAllocationStrategy::Random,
        }
    }
}

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
    Pooling {
        /// The allocation strategy to use.
        strategy: PoolingAllocationStrategy,
        /// The module limits to use.
        module_limits: ModuleLimits,
        /// The instance limits to use.
        instance_limits: InstanceLimits,
    },
}

impl InstanceAllocationStrategy {
    /// The default pooling instance allocation strategy.
    pub fn pooling() -> Self {
        Self::Pooling {
            strategy: PoolingAllocationStrategy::default(),
            module_limits: ModuleLimits::default(),
            instance_limits: InstanceLimits::default(),
        }
    }
}

impl Default for InstanceAllocationStrategy {
    fn default() -> Self {
        Self::OnDemand
    }
}

/// Global configuration options used to create an [`Engine`](crate::Engine)
/// and customize its behavior.
///
/// This structure exposed a builder-like interface and is primarily consumed by
/// [`Engine::new()`](crate::Engine::new)
#[derive(Clone)]
pub struct Config {
    pub(crate) flags: settings::Builder,
    pub(crate) isa_flags: isa::Builder,
    pub(crate) tunables: Tunables,
    pub(crate) strategy: CompilationStrategy,
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
    pub(crate) deserialize_check_wasmtime_version: bool,
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        let mut flags = settings::builder();

        // There are two possible traps for division, and this way
        // we get the proper one if code traps.
        flags
            .enable("avoid_div_traps")
            .expect("should be valid flag");

        // We don't use probestack as a stack limit mechanism
        flags
            .set("enable_probestack", "false")
            .expect("should be valid flag");

        let mut ret = Self {
            tunables: Tunables::default(),
            flags,
            isa_flags: native::builder(),
            strategy: CompilationStrategy::Auto,
            #[cfg(feature = "cache")]
            cache_config: CacheConfig::new_cache_disabled(),
            profiler: Arc::new(NullProfilerAgent),
            mem_creator: None,
            allocation_strategy: InstanceAllocationStrategy::OnDemand,
            max_wasm_stack: 1 << 20,
            wasm_backtrace_details_env_used: false,
            features: WasmFeatures::default(),
            #[cfg(feature = "async")]
            async_stack_size: 2 << 20,
            async_support: false,
            deserialize_check_wasmtime_version: true,
        };
        ret.cranelift_debug_verifier(false);
        ret.cranelift_opt_level(OptLevel::Speed);
        ret.wasm_reference_types(true);
        ret.wasm_multi_value(true);
        ret.wasm_bulk_memory(true);
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
    pub fn target(&mut self, target: &str) -> Result<&mut Self> {
        use std::str::FromStr;
        self.isa_flags = native::lookup(
            target_lexicon::Triple::from_str(target).map_err(|e| anyhow::anyhow!(e))?,
        )?;

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
    /// To remedy this situation you have a two possible ways to solve this:
    ///
    /// * First you can spawn futures into a thread pool. By doing this in a
    ///   thread pool you are relaxing the requirement that `Future::poll` must
    ///   be fast because your future is executing on a separate thread. This
    ///   strategy, however, would likely still require some form of
    ///   cancellation via [`crate::Store::interrupt_handle`] to ensure wasm
    ///   doesn't take *too* long to execute.
    ///
    /// * Alternatively you can enable the
    ///   [`Config::consume_fuel`](crate::Config::consume_fuel) method as well
    ///   as [`crate::Store::out_of_fuel_async_yield`] When doing so this will
    ///   configure Wasmtime futures to yield periodically while they're
    ///   executing WebAssembly code. After consuming the specified amount of
    ///   fuel wasm futures will return `Poll::Pending` from their `poll`
    ///   method, and will get automatically re-polled later. This enables the
    ///   `Future::poll` method to take roughly a fixed amount of time since
    ///   fuel is guaranteed to get consumed while wasm is executing. Note that
    ///   to prevent infinite execution of wasm you'll need to use either
    ///   [`crate::Store::interrupt_handle`] or a normal timeout on futures
    ///   (which will get triggered due to periodic `poll`s).
    ///
    /// In either case special care needs to be taken when integrating
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

    /// Configures whether functions and loops will be interruptable via the
    /// [`Store::interrupt_handle`](crate::Store::interrupt_handle) method.
    ///
    /// For more information see the documentation on
    /// [`Store::interrupt_handle`](crate::Store::interrupt_handle).
    ///
    /// By default this option is `false`.
    pub fn interruptable(&mut self, enable: bool) -> &mut Self {
        self.tunables.interruptable = enable;
        self
    }

    /// Configures whether execution of WebAssembly will "consume fuel" to
    /// either halt or yield execution as desired.
    ///
    /// This option is similar in purpose to [`Config::interruptable`] where
    /// you can prevent infinitely-executing WebAssembly code. The difference
    /// is that this option allows deterministic execution of WebAssembly code
    /// by instrumenting generated code consume fuel as it executes. When fuel
    /// runs out the behavior is defined by configuration within a [`Store`],
    /// and by default a trap is raised.
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
    /// By default this option is 1 MiB.
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
    /// This is `true` by default on x86-64, and `false` by default on other
    /// architectures.
    ///
    /// [proposal]: https://github.com/webassembly/reference-types
    pub fn wasm_reference_types(&mut self, enable: bool) -> &mut Self {
        self.features.reference_types = enable;

        self.flags
            .set("enable_safepoints", if enable { "true" } else { "false" })
            .unwrap();

        // The reference types proposal depends on the bulk memory proposal.
        if enable {
            self.wasm_bulk_memory(true);
        }

        self
    }

    /// Configures whether the WebAssembly SIMD proposal will be
    /// enabled for compilation.
    ///
    /// The [WebAssembly SIMD proposal][proposal] is not currently
    /// fully standardized and is undergoing development. Additionally the
    /// support in wasmtime itself is still being worked on. Support for this
    /// feature can be enabled through this method for appropriate wasm
    /// modules.
    ///
    /// This feature gates items such as the `v128` type and all of its
    /// operators being in a module.
    ///
    /// This is `false` by default.
    ///
    /// > **Note**: Wasmtime does not implement everything for the wasm simd
    /// > spec at this time, so bugs, panics, and possibly segfaults should be
    /// > expected. This should not be enabled in a production setting right
    /// > now.
    ///
    /// [proposal]: https://github.com/webassembly/simd
    pub fn wasm_simd(&mut self, enable: bool) -> &mut Self {
        self.features.simd = enable;
        let val = if enable { "true" } else { "false" };
        self.flags
            .set("enable_simd", val)
            .expect("should be valid flag");
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

    /// Configures whether the WebAssembly module linking [proposal] will
    /// be enabled for compilation.
    ///
    /// Note that development of this feature is still underway, so enabling
    /// this is likely to be full of bugs.
    ///
    /// This is `false` by default.
    ///
    /// [proposal]: https://github.com/webassembly/module-linking
    pub fn wasm_module_linking(&mut self, enable: bool) -> &mut Self {
        self.features.module_linking = enable;
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
    pub fn strategy(&mut self, strategy: Strategy) -> Result<&mut Self> {
        self.strategy = match strategy {
            Strategy::Auto => CompilationStrategy::Auto,
            Strategy::Cranelift => CompilationStrategy::Cranelift,
            #[cfg(feature = "lightbeam")]
            Strategy::Lightbeam => CompilationStrategy::Lightbeam,
            #[cfg(not(feature = "lightbeam"))]
            Strategy::Lightbeam => {
                anyhow::bail!("lightbeam compilation strategy wasn't enabled at compile time");
            }
        };
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
    pub fn cranelift_debug_verifier(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.flags
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
    pub fn cranelift_opt_level(&mut self, level: OptLevel) -> &mut Self {
        let val = match level {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        };
        self.flags
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
    pub fn cranelift_nan_canonicalization(&mut self, enable: bool) -> &mut Self {
        let val = if enable { "true" } else { "false" };
        self.flags
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
    pub unsafe fn cranelift_flag_enable(&mut self, flag: &str) -> Result<&mut Self> {
        if let Err(err) = self.flags.enable(flag) {
            match err {
                SetError::BadName(_) => {
                    // Try the target-specific flags.
                    self.isa_flags.enable(flag)?;
                }
                _ => bail!(err),
            }
        }
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
    pub unsafe fn cranelift_flag_set(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        if let Err(err) = self.flags.set(name, value) {
            match err {
                SetError::BadName(_) => {
                    // Try the target-specific flags.
                    self.isa_flags.set(name, value)?;
                }
                _ => bail!(err),
            }
        }
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
        self.tunables.static_memory_bound = u32::try_from(max_pages).unwrap_or(u32::max_value());
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

    /// Configure whether deserialized modules should validate version
    /// information. This only effects [`crate::Module::deserialize()`], which is
    /// used to load compiled code from trusted sources.  When true,
    /// [`crate::Module::deserialize()`] verifies that the wasmtime crate's
    /// `CARGO_PKG_VERSION` matches with the version in the binary, which was
    /// produced by [`crate::Module::serialize`] or
    /// [`crate::Engine::precompile_module`].
    ///
    /// This value defaults to true.
    pub fn deserialize_check_wasmtime_version(&mut self, check: bool) -> &mut Self {
        self.deserialize_check_wasmtime_version = check;
        self
    }

    pub(crate) fn target_isa(&self) -> Box<dyn TargetIsa> {
        self.isa_flags
            .clone()
            .finish(settings::Flags::new(self.flags.clone()))
    }

    pub(crate) fn target_isa_with_reference_types(&self) -> Box<dyn TargetIsa> {
        let mut flags = self.flags.clone();
        flags.set("enable_safepoints", "true").unwrap();
        self.isa_flags.clone().finish(settings::Flags::new(flags))
    }

    pub(crate) fn build_compiler(&self, allocator: &dyn InstanceAllocator) -> Compiler {
        let isa = self.target_isa();
        let mut tunables = self.tunables.clone();
        allocator.adjust_tunables(&mut tunables);
        Compiler::new(isa, self.strategy, tunables, self.features)
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
            InstanceAllocationStrategy::Pooling {
                strategy,
                module_limits,
                instance_limits,
            } => Ok(Box::new(PoolingInstanceAllocator::new(
                strategy.into(),
                module_limits.into(),
                instance_limits.into(),
                stack_size,
                &self.tunables,
            )?)),
        }
    }
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

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("debug_info", &self.tunables.generate_native_debuginfo)
            .field("parse_wasm_debuginfo", &self.tunables.parse_wasm_debuginfo)
            .field("strategy", &self.strategy)
            .field("wasm_threads", &self.features.threads)
            .field("wasm_reference_types", &self.features.reference_types)
            .field("wasm_bulk_memory", &self.features.bulk_memory)
            .field("wasm_simd", &self.features.simd)
            .field("wasm_multi_value", &self.features.multi_value)
            .field("wasm_module_linking", &self.features.module_linking)
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
            .field(
                "flags",
                &settings::Flags::new(self.flags.clone()).to_string(),
            )
            .finish()
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
    /// Currently this always defaults to Cranelift, but the default value will
    /// change over time.
    Auto,

    /// Currently the default backend, Cranelift aims to be a reasonably fast
    /// code generator which generates high quality machine code.
    Cranelift,

    /// A single-pass code generator that is faster than Cranelift but doesn't
    /// produce as high-quality code.
    ///
    /// To successfully pass this argument to [`Config::strategy`] the
    /// `lightbeam` feature of this crate must be enabled.
    Lightbeam,
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
