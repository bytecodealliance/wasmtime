//! Generate a configuration for both Wasmtime and the Wasm module to execute.

use super::{
    CodegenSettings, InstanceAllocationStrategy, MemoryConfig, ModuleConfig, NormalMemoryConfig,
    UnalignedMemoryCreator,
};
use crate::oracles::{StoreLimits, Timeout};
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use std::sync::Arc;
use std::time::Duration;
use wasmtime::{Engine, Module, Store};

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
    /// execution so only a single function should be generated since that's all
    /// that's going to be exercised.
    pub fn set_differential_config(&mut self) {
        let config = &mut self.module_config.config;

        config.allow_start_export = false;

        // Make sure there's a type available for the function.
        config.min_types = 1;
        config.max_types = config.max_types.max(1);

        // Generate at least one function
        config.min_funcs = 1;
        config.max_funcs = config.max_funcs.max(1);

        // Allow a memory to be generated, but don't let it get too large.
        // Additionally require the maximum size to guarantee that the growth
        // behavior is consistent across engines.
        config.max_memories = 1;
        config.max_memory_pages = 10;
        config.memory_max_size_required = true;

        // If tables are generated make sure they don't get too large to avoid
        // hitting any engine-specific limit. Additionally ensure that the
        // maximum size is required to guarantee consistent growth across
        // engines.
        //
        // Note that while reference types are disabled below, only allow one
        // table.
        config.max_tables = 1;
        config.max_table_elements = 1_000;
        config.table_max_size_required = true;

        // Don't allow any imports
        config.max_imports = 0;

        // Try to get the function and the memory exported
        config.export_everything = true;

        // NaN is canonicalized at the wasm level for differential fuzzing so we
        // can paper over NaN differences between engines.
        config.canonicalize_nans = true;

        // When diffing against a non-wasmtime engine then disable wasm
        // features to get selectively re-enabled against each differential
        // engine.
        config.bulk_memory_enabled = false;
        config.reference_types_enabled = false;
        config.simd_enabled = false;
        config.memory64_enabled = false;
        config.threads_enabled = false;

        // If using the pooling allocator, update the instance limits too
        if let InstanceAllocationStrategy::Pooling {
            instance_limits: limits,
            ..
        } = &mut self.wasmtime.strategy
        {
            // One single-page memory
            limits.memories = 1;
            limits.memory_pages = 10;

            limits.tables = 1;
            limits.table_elements = 1_000;

            match &mut self.wasmtime.memory_config {
                MemoryConfig::Normal(config) => {
                    config.static_memory_maximum_size = Some(limits.memory_pages * 0x10000);
                }
                MemoryConfig::CustomUnaligned => unreachable!(), // Arbitrary impl for `Config` should have prevented this
            }
        }
    }

    /// Uses this configuration and the supplied source of data to generate
    /// a wasm module.
    ///
    /// If a `default_fuel` is provided, the resulting module will be configured
    /// to ensure termination; as doing so will add an additional global to the module,
    /// the pooling allocator, if configured, will also have its globals limit updated.
    pub fn generate(
        &mut self,
        input: &mut Unstructured<'_>,
        default_fuel: Option<u32>,
    ) -> arbitrary::Result<wasm_smith::Module> {
        let mut module = wasm_smith::Module::new(self.module_config.config.clone(), input)?;

        if let Some(default_fuel) = default_fuel {
            module.ensure_termination(default_fuel);
        }

        Ok(module)
    }

    /// Indicates that this configuration should be spec-test-compliant,
    /// disabling various features the spec tests assert are disabled.
    pub fn set_spectest_compliant(&mut self) {
        let config = &mut self.module_config.config;
        config.memory64_enabled = false;
        config.bulk_memory_enabled = true;
        config.reference_types_enabled = true;
        config.multi_value_enabled = true;
        config.simd_enabled = true;
        config.threads_enabled = false;
        config.max_memories = 1;
        config.max_tables = 5;

        if let InstanceAllocationStrategy::Pooling {
            instance_limits: limits,
            ..
        } = &mut self.wasmtime.strategy
        {
            // Configure the lower bound of a number of limits to what's
            // required to actually run the spec tests. Fuzz-generated inputs
            // may have limits less than these thresholds which would cause the
            // spec tests to fail which isn't particularly interesting.
            limits.memories = limits.memories.max(1);
            limits.tables = limits.memories.max(5);
            limits.table_elements = limits.memories.max(1_000);
            limits.memory_pages = limits.memory_pages.max(900);
            limits.count = limits.count.max(500);
            limits.size = limits.size.max(64 * 1024);

            match &mut self.wasmtime.memory_config {
                MemoryConfig::Normal(config) => {
                    config.static_memory_maximum_size = Some(limits.memory_pages * 0x10000);
                }
                MemoryConfig::CustomUnaligned => unreachable!(), // Arbitrary impl for `Config` should have prevented this
            }
        }
    }

    /// Converts this to a `wasmtime::Config` object
    pub fn to_wasmtime(&self) -> wasmtime::Config {
        crate::init_fuzzing();
        log::debug!("creating wasmtime config with {:#?}", self.wasmtime);

        let mut cfg = wasmtime::Config::new();
        cfg.wasm_bulk_memory(true)
            .wasm_reference_types(true)
            .wasm_multi_value(self.module_config.config.multi_value_enabled)
            .wasm_multi_memory(self.module_config.config.max_memories > 1)
            .wasm_simd(self.module_config.config.simd_enabled)
            .wasm_memory64(self.module_config.config.memory64_enabled)
            .wasm_threads(self.module_config.config.threads_enabled)
            .wasm_backtrace(self.wasmtime.wasm_backtraces)
            .cranelift_nan_canonicalization(self.wasmtime.canonicalize_nans)
            .cranelift_opt_level(self.wasmtime.opt_level.to_wasmtime())
            .consume_fuel(self.wasmtime.consume_fuel)
            .epoch_interruption(self.wasmtime.epoch_interruption)
            .memory_init_cow(self.wasmtime.memory_init_cow)
            .memory_guaranteed_dense_image_size(std::cmp::min(
                // Clamp this at 16MiB so we don't get huge in-memory
                // images during fuzzing.
                16 << 20,
                self.wasmtime.memory_guaranteed_dense_image_size,
            ))
            .allocation_strategy(self.wasmtime.strategy.to_wasmtime())
            .generate_address_map(self.wasmtime.generate_address_map);

        self.wasmtime.codegen.configure(&mut cfg);

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
            match &self.wasmtime.memory_config {
                MemoryConfig::Normal(memory_config) => {
                    cfg.static_memory_maximum_size(
                        memory_config.static_memory_maximum_size.unwrap_or(0),
                    )
                    .static_memory_guard_size(memory_config.static_memory_guard_size.unwrap_or(0))
                    .dynamic_memory_guard_size(memory_config.dynamic_memory_guard_size.unwrap_or(0))
                    .guard_before_linear_memory(memory_config.guard_before_linear_memory);
                }
                MemoryConfig::CustomUnaligned => {
                    cfg.with_host_memory(Arc::new(UnalignedMemoryCreator))
                        .static_memory_maximum_size(0)
                        .dynamic_memory_guard_size(0)
                        .static_memory_guard_size(0)
                        .guard_before_linear_memory(false);
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
        if self.wasmtime.consume_fuel {
            store.add_fuel(u64::max_value()).unwrap();
        }
        if self.wasmtime.epoch_interruption {
            // Without fuzzing of async execution, we can't test the
            // "update deadline and continue" behavior, but we can at
            // least test the codegen paths and checks with the
            // trapping behavior, which works synchronously too. We'll
            // set the deadline one epoch tick in the future; then
            // this works exactly like an interrupt flag. We expect no
            // traps/interrupts unless we bump the epoch, which we do
            // as one particular Timeout mode (`Timeout::Epoch`).
            store.epoch_deadline_trap();
            store.set_epoch_deadline(1);
        }
    }

    /// Generates an arbitrary method of timing out an instance, ensuring that
    /// this configuration supports the returned timeout.
    pub fn generate_timeout(&mut self, u: &mut Unstructured<'_>) -> arbitrary::Result<Timeout> {
        let time_duration = Duration::from_secs(20);
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
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), module.serialize().unwrap()).unwrap();
        unsafe { Ok(Module::deserialize_file(engine, file.path()).unwrap()) }
    }
}

impl<'a> Arbitrary<'a> for Config {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut config = Self {
            wasmtime: u.arbitrary()?,
            module_config: u.arbitrary()?,
        };

        // If using the pooling allocator, constrain the memory and module configurations
        // to the module limits.
        if let InstanceAllocationStrategy::Pooling {
            instance_limits: limits,
            ..
        } = &config.wasmtime.strategy
        {
            // If the pooling allocator is used, do not allow shared memory to
            // be created. FIXME: see
            // https://github.com/bytecodealliance/wasmtime/issues/4244.
            config.module_config.config.threads_enabled = false;

            // Force the use of a normal memory config when using the pooling allocator and
            // limit the static memory maximum to be the same as the pooling allocator's memory
            // page limit.
            config.wasmtime.memory_config = match config.wasmtime.memory_config {
                MemoryConfig::Normal(mut config) => {
                    config.static_memory_maximum_size = Some(limits.memory_pages * 0x10000);
                    MemoryConfig::Normal(config)
                }
                MemoryConfig::CustomUnaligned => {
                    let mut config: NormalMemoryConfig = u.arbitrary()?;
                    config.static_memory_maximum_size = Some(limits.memory_pages * 0x10000);
                    MemoryConfig::Normal(config)
                }
            };

            let cfg = &mut config.module_config.config;
            cfg.max_memories = limits.memories as usize;
            cfg.max_tables = limits.tables as usize;
            cfg.max_memory_pages = limits.memory_pages;

            // Force no aliases in any generated modules as they might count against the
            // import limits above.
            cfg.max_aliases = 0;
        }

        Ok(config)
    }
}

/// Configuration related to `wasmtime::Config` and the various settings which
/// can be tweaked from within.
#[derive(Arbitrary, Clone, Debug, Eq, Hash, PartialEq)]
pub struct WasmtimeConfig {
    opt_level: OptLevel,
    debug_info: bool,
    canonicalize_nans: bool,
    interruptable: bool,
    pub(crate) consume_fuel: bool,
    epoch_interruption: bool,
    /// The Wasmtime memory configuration to use.
    pub memory_config: MemoryConfig,
    force_jump_veneers: bool,
    memory_init_cow: bool,
    memory_guaranteed_dense_image_size: u64,
    use_precompiled_cwasm: bool,
    /// Configuration for the instance allocation strategy to use.
    pub strategy: InstanceAllocationStrategy,
    codegen: CodegenSettings,
    padding_between_functions: Option<u16>,
    generate_address_map: bool,
    wasm_backtraces: bool,
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
