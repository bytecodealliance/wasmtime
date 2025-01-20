//! Generate a configuration for both Wasmtime and the Wasm module to execute.

use super::{
    AsyncConfig, CodegenSettings, InstanceAllocationStrategy, MemoryConfig, ModuleConfig,
    NormalMemoryConfig, UnalignedMemoryCreator,
};
use crate::oracles::{StoreLimits, Timeout};
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use std::sync::Arc;
use std::time::Duration;
use wasmtime::{Engine, Module, MpkEnabled, Store};
use wasmtime_wast_util::{limits, WastConfig, WastTest};

/// Configuration for `wasmtime::Config` and generated modules for a session of
/// fuzzing.
///
/// This configuration guides what modules are generated, how wasmtime
/// configuration is generated, and is typically itself generated through a call
/// to `Arbitrary` which allows for a form of "swarm testing".
#[derive(Debug, Clone)]
pub struct Config {
    /// Configuration related to the `wasmtime::Config`.
    pub wasmtime: WasmtimeConfig,
    /// Configuration related to generated modules.
    pub module_config: ModuleConfig,
}

impl Config {
    /// Indicates that this configuration is being used for differential
    /// execution.
    ///
    /// The purpose of this function is to update the configuration which was
    /// generated to be compatible with execution in multiple engines. The goal
    /// is to produce the exact same result in all engines so we need to paper
    /// over things like nan differences and memory/table behavior differences.
    pub fn set_differential_config(&mut self) {
        let config = &mut self.module_config.config;

        // Make it more likely that there are types available to generate a
        // function with.
        config.min_types = config.min_types.max(1);
        config.max_types = config.max_types.max(1);

        // Generate at least one function
        config.min_funcs = config.min_funcs.max(1);
        config.max_funcs = config.max_funcs.max(1);

        // Allow a memory to be generated, but don't let it get too large.
        // Additionally require the maximum size to guarantee that the growth
        // behavior is consistent across engines.
        config.max_memory32_bytes = 10 << 16;
        config.max_memory64_bytes = 10 << 16;
        config.memory_max_size_required = true;

        // If tables are generated make sure they don't get too large to avoid
        // hitting any engine-specific limit. Additionally ensure that the
        // maximum size is required to guarantee consistent growth across
        // engines.
        //
        // Note that while reference types are disabled below, only allow one
        // table.
        config.max_table_elements = 1_000;
        config.table_max_size_required = true;

        // Don't allow any imports
        config.max_imports = 0;

        // Try to get the function and the memory exported
        config.export_everything = true;

        // NaN is canonicalized at the wasm level for differential fuzzing so we
        // can paper over NaN differences between engines.
        config.canonicalize_nans = true;

        // If using the pooling allocator, update the instance limits too
        if let InstanceAllocationStrategy::Pooling(pooling) = &mut self.wasmtime.strategy {
            // One single-page memory
            pooling.total_memories = config.max_memories as u32;
            pooling.max_memory_size = 10 << 16;
            pooling.max_memories_per_module = config.max_memories as u32;
            if pooling.memory_protection_keys == MpkEnabled::Auto
                && pooling.max_memory_protection_keys > 1
            {
                pooling.total_memories =
                    pooling.total_memories * (pooling.max_memory_protection_keys as u32);
            }

            pooling.total_tables = config.max_tables as u32;
            pooling.table_elements = 1_000;
            pooling.max_tables_per_module = config.max_tables as u32;

            pooling.core_instance_size = 1_000_000;

            if let MemoryConfig::Normal(cfg) = &mut self.wasmtime.memory_config {
                match &mut cfg.memory_reservation {
                    Some(size) => *size = (*size).max(pooling.max_memory_size as u64),
                    other @ None => *other = Some(pooling.max_memory_size as u64),
                }
            }
        }

        // These instructions are explicitly not expected to be exactly the same
        // across engines. Don't fuzz them.
        config.relaxed_simd_enabled = false;
    }

    /// Uses this configuration and the supplied source of data to generate
    /// a wasm module.
    ///
    /// If a `default_fuel` is provided, the resulting module will be configured
    /// to ensure termination; as doing so will add an additional global to the module,
    /// the pooling allocator, if configured, will also have its globals limit updated.
    pub fn generate(
        &self,
        input: &mut Unstructured<'_>,
        default_fuel: Option<u32>,
    ) -> arbitrary::Result<wasm_smith::Module> {
        self.module_config.generate(input, default_fuel)
    }

    /// Updates this configuration to be able to run the `test` specified.
    ///
    /// This primarily updates `self.module_config` to ensure that it enables
    /// all features and proposals necessary to execute the `test` specified.
    /// This will additionally update limits in the pooling allocator to be able
    /// to execute all tests.
    pub fn make_wast_test_compliant(&mut self, test: &WastTest) -> WastConfig {
        let wasmtime_wast_util::TestConfig {
            memory64,
            custom_page_sizes,
            multi_memory,
            threads,
            gc,
            function_references,
            relaxed_simd,
            reference_types,
            tail_call,
            extended_const,
            wide_arithmetic,
            component_model_more_flags,
            simd,

            hogs_memory: _,
            nan_canonicalization: _,
            gc_types: _,
        } = test.config;

        // Enable/disable some proposals that aren't configurable in wasm-smith
        // but are configurable in Wasmtime.
        self.module_config.function_references_enabled =
            function_references.or(gc).unwrap_or(false);
        self.module_config.component_model_more_flags = component_model_more_flags.unwrap_or(false);

        // Enable/disable proposals that wasm-smith has knobs for which will be
        // read when creating `wasmtime::Config`.
        let config = &mut self.module_config.config;
        config.bulk_memory_enabled = true;
        config.multi_value_enabled = true;
        config.wide_arithmetic_enabled = wide_arithmetic.unwrap_or(false);
        config.memory64_enabled = memory64.unwrap_or(false);
        config.relaxed_simd_enabled = relaxed_simd.unwrap_or(false);
        config.simd_enabled = config.relaxed_simd_enabled || simd.unwrap_or(false);
        config.tail_call_enabled = tail_call.unwrap_or(false);
        config.custom_page_sizes_enabled = custom_page_sizes.unwrap_or(false);
        config.threads_enabled = threads.unwrap_or(false);
        config.gc_enabled = gc.unwrap_or(false);
        config.reference_types_enabled = config.gc_enabled
            || self.module_config.function_references_enabled
            || reference_types.unwrap_or(false);
        config.extended_const_enabled = extended_const.unwrap_or(false);
        if multi_memory.unwrap_or(false) {
            config.max_memories = limits::MEMORIES_PER_MODULE as usize;
        } else {
            config.max_memories = 1;
        }

        match &mut self.wasmtime.memory_config {
            MemoryConfig::Normal(config) => {
                if let Some(n) = &mut config.memory_reservation {
                    *n = (*n).max(limits::MEMORY_SIZE as u64);
                }
            }
            MemoryConfig::CustomUnaligned => {}
        }

        // FIXME: it might be more ideal to avoid the need for this entirely
        // and to just let the test fail. If a test fails due to a pooling
        // allocator resource limit being met we could ideally detect that and
        // let the fuzz test case pass. That would avoid the need to hardcode
        // so much here and in theory wouldn't reduce the usefulness of fuzzers
        // all that much. At this time though we can't easily test this configuration.
        if let InstanceAllocationStrategy::Pooling(pooling) = &mut self.wasmtime.strategy {
            // Clamp protection keys between 1 & 2 to reduce the number of
            // slots and then multiply the total memories by the number of keys
            // we have since a single store has access to only one key.
            pooling.max_memory_protection_keys = pooling.max_memory_protection_keys.max(1).min(2);
            pooling.total_memories = pooling
                .total_memories
                .max(limits::MEMORIES * (pooling.max_memory_protection_keys as u32));

            // For other limits make sure they meet the minimum threshold
            // required for our wast tests.
            pooling.total_component_instances = pooling
                .total_component_instances
                .max(limits::COMPONENT_INSTANCES);
            pooling.total_tables = pooling.total_tables.max(limits::TABLES);
            pooling.max_tables_per_module =
                pooling.max_tables_per_module.max(limits::TABLES_PER_MODULE);
            pooling.max_memories_per_module = pooling
                .max_memories_per_module
                .max(limits::MEMORIES_PER_MODULE);
            pooling.max_memories_per_component = pooling
                .max_memories_per_component
                .max(limits::MEMORIES_PER_MODULE);
            pooling.total_core_instances = pooling.total_core_instances.max(limits::CORE_INSTANCES);
            pooling.max_memory_size = pooling.max_memory_size.max(limits::MEMORY_SIZE);
            pooling.table_elements = pooling.table_elements.max(limits::TABLE_ELEMENTS);
            pooling.core_instance_size = pooling.core_instance_size.max(limits::CORE_INSTANCE_SIZE);
            pooling.component_instance_size = pooling
                .component_instance_size
                .max(limits::CORE_INSTANCE_SIZE);
        }

        // Return the test configuration that this fuzz configuration represents
        // which is used afterwards to test if the `test` here is expected to
        // fail or not.
        WastConfig {
            collector: match self.wasmtime.collector {
                Collector::Null => wasmtime_wast_util::Collector::Null,
                Collector::DeferredReferenceCounting => {
                    wasmtime_wast_util::Collector::DeferredReferenceCounting
                }
            },
            pooling: matches!(
                self.wasmtime.strategy,
                InstanceAllocationStrategy::Pooling(_)
            ),
            compiler: match self.wasmtime.compiler_strategy {
                CompilerStrategy::CraneliftNative => wasmtime_wast_util::Compiler::CraneliftNative,
                CompilerStrategy::CraneliftPulley => wasmtime_wast_util::Compiler::CraneliftPulley,
                CompilerStrategy::Winch => wasmtime_wast_util::Compiler::Winch,
            },
        }
    }

    /// Converts this to a `wasmtime::Config` object
    pub fn to_wasmtime(&self) -> wasmtime::Config {
        crate::init_fuzzing();
        log::debug!("creating wasmtime config with {:#?}", self.wasmtime);

        let mut cfg = wasmtime::Config::new();
        cfg.parallel_compilation(false)
            .wasm_bulk_memory(true)
            .wasm_reference_types(self.module_config.config.reference_types_enabled)
            .wasm_multi_value(self.module_config.config.multi_value_enabled)
            .wasm_multi_memory(self.module_config.config.max_memories > 1)
            .wasm_simd(self.module_config.config.simd_enabled)
            .wasm_memory64(self.module_config.config.memory64_enabled)
            .wasm_tail_call(self.module_config.config.tail_call_enabled)
            .wasm_custom_page_sizes(self.module_config.config.custom_page_sizes_enabled)
            .wasm_threads(self.module_config.config.threads_enabled)
            .wasm_function_references(self.module_config.function_references_enabled)
            .wasm_gc(self.module_config.config.gc_enabled)
            .wasm_custom_page_sizes(self.module_config.config.custom_page_sizes_enabled)
            .wasm_wide_arithmetic(self.module_config.config.wide_arithmetic_enabled)
            .wasm_extended_const(self.module_config.config.extended_const_enabled)
            .wasm_component_model_more_flags(self.module_config.component_model_more_flags)
            .native_unwind_info(cfg!(target_os = "windows") || self.wasmtime.native_unwind_info)
            .cranelift_nan_canonicalization(self.wasmtime.canonicalize_nans)
            .cranelift_opt_level(self.wasmtime.opt_level.to_wasmtime())
            .cranelift_regalloc_algorithm(self.wasmtime.regalloc_algorithm.to_wasmtime())
            .consume_fuel(self.wasmtime.consume_fuel)
            .epoch_interruption(self.wasmtime.epoch_interruption)
            .memory_guaranteed_dense_image_size(std::cmp::min(
                // Clamp this at 16MiB so we don't get huge in-memory
                // images during fuzzing.
                16 << 20,
                self.wasmtime.memory_guaranteed_dense_image_size,
            ))
            .allocation_strategy(self.wasmtime.strategy.to_wasmtime())
            .generate_address_map(self.wasmtime.generate_address_map)
            .signals_based_traps(self.wasmtime.signals_based_traps)
            .async_stack_zeroing(self.wasmtime.async_stack_zeroing);

        if !self.module_config.config.simd_enabled {
            cfg.wasm_relaxed_simd(false);
        }

        let compiler_strategy = &self.wasmtime.compiler_strategy;
        let cranelift_strategy = match compiler_strategy {
            CompilerStrategy::CraneliftNative | CompilerStrategy::CraneliftPulley => true,
            CompilerStrategy::Winch => false,
        };
        self.wasmtime.compiler_strategy.configure(&mut cfg);
        cfg.collector(self.wasmtime.collector.to_wasmtime());

        self.wasmtime.codegen.configure(&mut cfg);

        // Determine whether we will actually enable PCC -- this is
        // disabled if the module requires memory64, which is not yet
        // compatible (due to the need for dynamic checks).
        let pcc = cfg!(feature = "fuzz-pcc")
            && self.wasmtime.pcc
            && !self.module_config.config.memory64_enabled;

        // Only set cranelift specific flags when the Cranelift strategy is
        // chosen.
        if cranelift_strategy {
            // If the wasm-smith-generated module use nan canonicalization then we
            // don't need to enable it, but if it doesn't enable it already then we
            // enable this codegen option.
            cfg.cranelift_nan_canonicalization(!self.module_config.config.canonicalize_nans);

            // Enabling the verifier will at-least-double compilation time, which
            // with a 20-30x slowdown in fuzzing can cause issues related to
            // timeouts. If generated modules can have more than a small handful of
            // functions then disable the verifier when fuzzing to try to lessen the
            // impact of timeouts.
            if self.module_config.config.max_funcs > 10 {
                cfg.cranelift_debug_verifier(false);
            }

            if self.wasmtime.force_jump_veneers {
                unsafe {
                    cfg.cranelift_flag_set("wasmtime_linkopt_force_jump_veneer", "true");
                }
            }

            if let Some(pad) = self.wasmtime.padding_between_functions {
                unsafe {
                    cfg.cranelift_flag_set(
                        "wasmtime_linkopt_padding_between_functions",
                        &pad.to_string(),
                    );
                }
            }

            cfg.cranelift_pcc(pcc);

            // Eager init is currently only supported on Cranelift, not Winch.
            cfg.table_lazy_init(self.wasmtime.table_lazy_init);
        }

        self.wasmtime.async_config.configure(&mut cfg);

        // Vary the memory configuration, but only if threads are not enabled.
        // When the threads proposal is enabled we might generate shared memory,
        // which is less amenable to different memory configurations:
        // - shared memories are required to be "static" so fuzzing the various
        //   memory configurations will mostly result in uninteresting errors.
        //   The interesting part about shared memories is the runtime so we
        //   don't fuzz non-default settings.
        // - shared memories are required to be aligned which means that the
        //   `CustomUnaligned` variant isn't actually safe to use with a shared
        //   memory.
        if !self.module_config.config.threads_enabled {
            // If PCC is enabled, force other options to be compatible: PCC is currently only
            // supported when bounds checks are elided.
            let memory_config = if pcc {
                MemoryConfig::Normal(NormalMemoryConfig {
                    memory_reservation: Some(4 << 30), // 4 GiB
                    memory_guard_size: Some(2 << 30),  // 2 GiB
                    memory_reservation_for_growth: Some(0),
                    guard_before_linear_memory: false,
                    memory_init_cow: true,
                    // Doesn't matter, only using virtual memory.
                    cranelift_enable_heap_access_spectre_mitigations: None,
                })
            } else {
                self.wasmtime.memory_config.clone()
            };

            match &memory_config {
                MemoryConfig::Normal(memory_config) => {
                    memory_config.apply_to(&mut cfg);
                }
                MemoryConfig::CustomUnaligned => {
                    cfg.with_host_memory(Arc::new(UnalignedMemoryCreator))
                        .memory_reservation(0)
                        .memory_guard_size(0)
                        .memory_reservation_for_growth(0)
                        .guard_before_linear_memory(false)
                        .memory_init_cow(false);
                }
            }
        }

        if self.wasmtime.compiler_strategy == CompilerStrategy::Winch
            && self.module_config.config.simd_enabled
        {
            // Keep AVX and AVX2 matching host support. Otherwise SIMD fuzzing
            // breaks because there is no support for SIMD without AVX and AVX2.
            unsafe {
                if std::is_x86_feature_detected!("avx") {
                    cfg.cranelift_flag_enable("has_avx");
                }
                if std::is_x86_feature_detected!("avx2") {
                    cfg.cranelift_flag_enable("has_avx2");
                }
            }
        }

        return cfg;
    }

    /// Convenience function for generating a `Store<T>` using this
    /// configuration.
    pub fn to_store(&self) -> Store<StoreLimits> {
        let engine = Engine::new(&self.to_wasmtime()).unwrap();
        let mut store = Store::new(&engine, StoreLimits::new());
        self.configure_store(&mut store);
        store
    }

    /// Configures a store based on this configuration.
    pub fn configure_store(&self, store: &mut Store<StoreLimits>) {
        store.limiter(|s| s as &mut dyn wasmtime::ResourceLimiter);
        match self.wasmtime.async_config {
            AsyncConfig::Disabled => {
                if self.wasmtime.consume_fuel {
                    store.set_fuel(u64::MAX).unwrap();
                }
                if self.wasmtime.epoch_interruption {
                    store.epoch_deadline_trap();
                    store.set_epoch_deadline(1);
                }
            }
            AsyncConfig::YieldWithFuel(amt) => {
                assert!(self.wasmtime.consume_fuel);
                store.fuel_async_yield_interval(Some(amt)).unwrap();
                store.set_fuel(amt).unwrap();
            }
            AsyncConfig::YieldWithEpochs { ticks, .. } => {
                assert!(self.wasmtime.epoch_interruption);
                store.set_epoch_deadline(ticks);
                store.epoch_deadline_async_yield_and_update(ticks);
            }
        }
    }

    /// Generates an arbitrary method of timing out an instance, ensuring that
    /// this configuration supports the returned timeout.
    pub fn generate_timeout(&mut self, u: &mut Unstructured<'_>) -> arbitrary::Result<Timeout> {
        let time_duration = Duration::from_millis(100);
        let timeout = u
            .choose(&[Timeout::Fuel(100_000), Timeout::Epoch(time_duration)])?
            .clone();
        match &timeout {
            Timeout::Fuel(..) => {
                self.wasmtime.consume_fuel = true;
            }
            Timeout::Epoch(..) => {
                self.wasmtime.epoch_interruption = true;
            }
            Timeout::None => unreachable!("Not an option given to choose()"),
        }
        Ok(timeout)
    }

    /// Compiles the `wasm` within the `engine` provided.
    ///
    /// This notably will use `Module::{serialize,deserialize_file}` to
    /// round-trip if configured in the fuzzer.
    pub fn compile(&self, engine: &Engine, wasm: &[u8]) -> Result<Module> {
        // Propagate this error in case the caller wants to handle
        // valid-vs-invalid wasm.
        let module = Module::new(engine, wasm)?;
        if !self.wasmtime.use_precompiled_cwasm {
            return Ok(module);
        }

        // Don't propagate these errors to prevent them from accidentally being
        // interpreted as invalid wasm, these should never fail on a
        // well-behaved host system.
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("module.wasm");
        std::fs::write(&file, module.serialize().unwrap()).unwrap();
        unsafe { Ok(Module::deserialize_file(engine, &file).unwrap()) }
    }

    /// Updates this configuration to forcibly enable async support. Only useful
    /// in fuzzers which do async calls.
    pub fn enable_async(&mut self, u: &mut Unstructured<'_>) -> arbitrary::Result<()> {
        if self.wasmtime.consume_fuel || u.arbitrary()? {
            self.wasmtime.async_config =
                AsyncConfig::YieldWithFuel(u.int_in_range(1000..=100_000)?);
            self.wasmtime.consume_fuel = true;
        } else {
            self.wasmtime.async_config = AsyncConfig::YieldWithEpochs {
                dur: Duration::from_millis(u.int_in_range(1..=10)?),
                ticks: u.int_in_range(1..=10)?,
            };
            self.wasmtime.epoch_interruption = true;
        }
        Ok(())
    }
}

impl<'a> Arbitrary<'a> for Config {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut config = Self {
            wasmtime: u.arbitrary()?,
            module_config: u.arbitrary()?,
        };

        config
            .wasmtime
            .update_module_config(&mut config.module_config.config, u)?;

        Ok(config)
    }
}

/// Configuration related to `wasmtime::Config` and the various settings which
/// can be tweaked from within.
#[derive(Arbitrary, Clone, Debug, Eq, Hash, PartialEq)]
pub struct WasmtimeConfig {
    opt_level: OptLevel,
    regalloc_algorithm: RegallocAlgorithm,
    debug_info: bool,
    canonicalize_nans: bool,
    interruptable: bool,
    pub(crate) consume_fuel: bool,
    pub(crate) epoch_interruption: bool,
    /// The Wasmtime memory configuration to use.
    pub memory_config: MemoryConfig,
    force_jump_veneers: bool,
    memory_init_cow: bool,
    memory_guaranteed_dense_image_size: u64,
    use_precompiled_cwasm: bool,
    async_stack_zeroing: bool,
    /// Configuration for the instance allocation strategy to use.
    pub strategy: InstanceAllocationStrategy,
    codegen: CodegenSettings,
    padding_between_functions: Option<u16>,
    generate_address_map: bool,
    native_unwind_info: bool,
    /// Configuration for the compiler to use.
    pub compiler_strategy: CompilerStrategy,
    collector: Collector,
    table_lazy_init: bool,

    /// Whether or not fuzzing should enable PCC.
    pcc: bool,

    /// Configuration for whether wasm is invoked in an async fashion and how
    /// it's cooperatively time-sliced.
    pub async_config: AsyncConfig,

    /// Whether or not host signal handlers are enabled for this configuration,
    /// aka whether signal handlers are supported.
    signals_based_traps: bool,
}

impl WasmtimeConfig {
    /// Force `self` to be a configuration compatible with `other`. This is
    /// useful for differential execution to avoid unhelpful fuzz crashes when
    /// one engine has a feature enabled and the other does not.
    pub fn make_compatible_with(&mut self, other: &Self) {
        // Use the same allocation strategy between the two configs.
        //
        // Ideally this wouldn't be necessary, but, during differential
        // evaluation, if the `lhs` is using ondemand and the `rhs` is using the
        // pooling allocator (or vice versa), then the module may have been
        // generated in such a way that is incompatible with the other
        // allocation strategy.
        //
        // We can remove this in the future when it's possible to access the
        // fields of `wasm_smith::Module` to constrain the pooling allocator
        // based on what was actually generated.
        self.strategy = other.strategy.clone();
        if let InstanceAllocationStrategy::Pooling { .. } = &other.strategy {
            // Also use the same memory configuration when using the pooling
            // allocator.
            self.memory_config = other.memory_config.clone();
        }

        self.make_internally_consistent();
    }

    /// Updates `config` to be compatible with `self` and the other way around
    /// too.
    pub fn update_module_config(
        &mut self,
        config: &mut wasm_smith::Config,
        u: &mut Unstructured<'_>,
    ) -> arbitrary::Result<()> {
        match self.compiler_strategy {
            CompilerStrategy::CraneliftNative => {}

            // Winch doesn't support the same set of wasm proposal as Cranelift
            // at this time, so if winch is selected be sure to disable wasm
            // proposals in `Config` to ensure that Winch can compile the
            // module that wasm-smith generates.
            CompilerStrategy::Winch => {
                config.simd_enabled = false;
                config.relaxed_simd_enabled = false;
                config.gc_enabled = false;
                config.threads_enabled = false;
                config.tail_call_enabled = false;
                config.reference_types_enabled = false;

                // Tuning  the following engine options is currently not supported
                // by Winch.
                self.signals_based_traps = true;
                self.table_lazy_init = true;
                self.debug_info = false;
            }

            CompilerStrategy::CraneliftPulley => {
                config.threads_enabled = false;
            }
        }

        // Forcibly don't use the `CustomUnaligned` memory configuration when
        // wasm threads are enabled or when the pooling allocator is used. For
        // the pooling allocator it doesn't use custom memory creators anyway
        // and for wasm threads that will require some refactoring of the
        // `LinearMemory` trait to bubble up the request that the linear memory
        // not move. Otherwise that just generates a panic right now.
        if config.threads_enabled || matches!(self.strategy, InstanceAllocationStrategy::Pooling(_))
        {
            self.avoid_custom_unaligned_memory(u)?;
        }

        // If using the pooling allocator, constrain the memory and module configurations
        // to the module limits.
        if let InstanceAllocationStrategy::Pooling(pooling) = &mut self.strategy {
            // If the pooling allocator is used, do not allow shared memory to
            // be created. FIXME: see
            // https://github.com/bytecodealliance/wasmtime/issues/4244.
            config.threads_enabled = false;

            // Ensure the pooling allocator can support the maximal size of
            // memory, picking the smaller of the two to win.
            let min_bytes = config
                .max_memory32_bytes
                // memory64_bytes is a u128, but since we are taking the min
                // we can truncate it down to a u64.
                .min(config.max_memory64_bytes.try_into().unwrap_or(u64::MAX));
            let mut min = min_bytes.min(pooling.max_memory_size as u64);
            if let MemoryConfig::Normal(cfg) = &self.memory_config {
                min = min.min(cfg.memory_reservation.unwrap_or(0));
            }
            pooling.max_memory_size = min as usize;
            config.max_memory32_bytes = min;
            config.max_memory64_bytes = min as u128;

            // If traps are disallowed then memories must have at least one page
            // of memory so if we still are only allowing 0 pages of memory then
            // increase that to one here.
            if config.disallow_traps {
                if pooling.max_memory_size < (1 << 16) {
                    pooling.max_memory_size = 1 << 16;
                    config.max_memory32_bytes = 1 << 16;
                    config.max_memory64_bytes = 1 << 16;
                    if let MemoryConfig::Normal(cfg) = &mut self.memory_config {
                        match &mut cfg.memory_reservation {
                            Some(size) => *size = (*size).max(pooling.max_memory_size as u64),
                            size @ None => *size = Some(pooling.max_memory_size as u64),
                        }
                    }
                }
                // .. additionally update tables
                if pooling.table_elements == 0 {
                    pooling.table_elements = 1;
                }
            }

            // Don't allow too many linear memories per instance since massive
            // virtual mappings can fail to get allocated.
            config.min_memories = config.min_memories.min(10);
            config.max_memories = config.max_memories.min(10);

            // Force this pooling allocator to always be able to accommodate the
            // module that may be generated.
            pooling.total_memories = config.max_memories as u32;
            pooling.total_tables = config.max_tables as u32;
        }

        if !self.signals_based_traps {
            // At this time shared memories require a "static" memory
            // configuration but when signals-based traps are disabled all
            // memories are forced to the "dynamic" configuration. This is
            // fixable with some more work on the bounds-checks side of things
            // to do a full bounds check even on static memories, but that's
            // left for a future PR.
            config.threads_enabled = false;

            // Spectre-based heap mitigations require signal handlers so this
            // must always be disabled if signals-based traps are disabled.
            if let MemoryConfig::Normal(cfg) = &mut self.memory_config {
                cfg.cranelift_enable_heap_access_spectre_mitigations = None;
            }
        }

        self.make_internally_consistent();

        Ok(())
    }

    /// Helper to switch `MemoryConfig::CustomUnaligned` to
    /// `MemoryConfig::Normal`
    fn avoid_custom_unaligned_memory(&mut self, u: &mut Unstructured<'_>) -> arbitrary::Result<()> {
        if let MemoryConfig::CustomUnaligned = self.memory_config {
            self.memory_config = MemoryConfig::Normal(u.arbitrary()?);
        }
        Ok(())
    }

    /// Helper method to handle some dependencies between various configuration
    /// options. This is intended to be called whenever a `Config` is created or
    /// modified to ensure that the final result is an instantiable `Config`.
    ///
    /// Note that in general this probably shouldn't exist and anything here can
    /// be considered a "TODO" to go implement more stuff in Wasmtime to accept
    /// these sorts of configurations. For now though it's intended to reflect
    /// the current state of the engine's development.
    fn make_internally_consistent(&mut self) {
        if !self.signals_based_traps {
            if let MemoryConfig::Normal(cfg) = &mut self.memory_config {
                // Spectre-based heap mitigations require signal handlers so
                // this must always be disabled if signals-based traps are
                // disabled.
                cfg.cranelift_enable_heap_access_spectre_mitigations = None;

                // With configuration settings that match the use of malloc for
                // linear memories cap the `memory_reservation_for_growth` value
                // to something reasonable to avoid OOM in fuzzing.
                if !cfg.memory_init_cow
                    && cfg.memory_guard_size == Some(0)
                    && cfg.memory_reservation == Some(0)
                {
                    let min = 10 << 20; // 10 MiB
                    if let Some(val) = &mut cfg.memory_reservation_for_growth {
                        *val = (*val).min(min);
                    } else {
                        cfg.memory_reservation_for_growth = Some(min);
                    }
                }
            }
        }
    }
}

#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
enum OptLevel {
    None,
    Speed,
    SpeedAndSize,
}

impl OptLevel {
    fn to_wasmtime(&self) -> wasmtime::OptLevel {
        match self {
            OptLevel::None => wasmtime::OptLevel::None,
            OptLevel::Speed => wasmtime::OptLevel::Speed,
            OptLevel::SpeedAndSize => wasmtime::OptLevel::SpeedAndSize,
        }
    }
}

#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
enum RegallocAlgorithm {
    Backtracking,
    SinglePass,
}

impl RegallocAlgorithm {
    fn to_wasmtime(&self) -> wasmtime::RegallocAlgorithm {
        match self {
            RegallocAlgorithm::Backtracking => wasmtime::RegallocAlgorithm::Backtracking,
            RegallocAlgorithm::SinglePass => wasmtime::RegallocAlgorithm::SinglePass,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Compiler to use.
pub enum CompilerStrategy {
    /// Cranelift compiler for the native architecture.
    CraneliftNative,
    /// Winch compiler.
    Winch,
    /// Cranelift compiler for the native architecture.
    CraneliftPulley,
}

impl CompilerStrategy {
    /// Configures `config` to use this compilation strategy
    pub fn configure(&self, config: &mut wasmtime::Config) {
        match self {
            CompilerStrategy::CraneliftNative => {
                config.strategy(wasmtime::Strategy::Cranelift);
            }
            CompilerStrategy::Winch => {
                config.strategy(wasmtime::Strategy::Winch);
            }
            CompilerStrategy::CraneliftPulley => {
                config
                    .strategy(wasmtime::Strategy::Cranelift)
                    .target("pulley64")
                    .unwrap();
            }
        }
    }
}

impl Arbitrary<'_> for CompilerStrategy {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        // Favor fuzzing native cranelift, but if allowed also enable
        // winch/pulley.
        match u.int_in_range(0..=19)? {
            1 => Ok(Self::CraneliftPulley),
            2 => Ok(Self::Winch),
            _ => Ok(Self::CraneliftNative),
        }
    }
}

#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Collector {
    DeferredReferenceCounting,
    Null,
}

impl Collector {
    fn to_wasmtime(&self) -> wasmtime::Collector {
        match self {
            Collector::DeferredReferenceCounting => wasmtime::Collector::DeferredReferenceCounting,
            Collector::Null => wasmtime::Collector::Null,
        }
    }
}
