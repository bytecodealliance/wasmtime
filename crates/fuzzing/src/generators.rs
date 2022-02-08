//! Test case generators.
//!
//! Test case generators take raw, unstructured input from a fuzzer
//! (e.g. libFuzzer) and translate that into a structured test case (e.g. a
//! valid Wasm binary).
//!
//! These are generally implementations of the `Arbitrary` trait, or some
//! wrapper over an external tool, such that the wrapper implements the
//! `Arbitrary` trait for the wrapped external tool.

pub mod api;
pub mod table_ops;

use crate::oracles::{StoreLimits, Timeout};
use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use std::sync::Arc;
use std::time::Duration;
use wasm_smith::SwarmConfig;
use wasmtime::{Engine, LinearMemory, MemoryCreator, MemoryType, Store};

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

/// Configuration for `wasmtime::Config` and generated modules for a session of
/// fuzzing.
///
/// This configuration guides what modules are generated, how wasmtime
/// configuration is generated, and is typically itself generated through a call
/// to `Arbitrary` which allows for a form of "swarm testing".
#[derive(Arbitrary, Debug)]
pub struct Config {
    /// Configuration related to the `wasmtime::Config`.
    pub wasmtime: WasmtimeConfig,
    /// Configuration related to generated modules.
    pub module_config: ModuleConfig,
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
    memory_config: MemoryConfig,
    force_jump_veneers: bool,
    memfd: bool,
}

#[derive(Arbitrary, Clone, Debug, Eq, Hash, PartialEq)]
enum MemoryConfig {
    /// Configuration for linear memories which correspond to normal
    /// configuration settings in `wasmtime` itself. This will tweak various
    /// parameters about static/dynamic memories.
    ///
    /// Note that we use 32-bit values here to avoid blowing the 64-bit address
    /// space by requesting ungodly-large sizes/guards.
    Normal {
        static_memory_maximum_size: Option<u32>,
        static_memory_guard_size: Option<u32>,
        dynamic_memory_guard_size: Option<u32>,
        guard_before_linear_memory: bool,
    },

    /// Configuration to force use of a linear memory that's unaligned at its
    /// base address to force all wasm addresses to be unaligned at the hardware
    /// level, even if the wasm itself correctly aligns everything internally.
    CustomUnaligned,
}

impl Config {
    /// Converts this to a `wasmtime::Config` object
    pub fn to_wasmtime(&self) -> wasmtime::Config {
        crate::init_fuzzing();

        let mut cfg = wasmtime::Config::new();
        cfg.wasm_bulk_memory(true)
            .wasm_reference_types(true)
            .wasm_module_linking(self.module_config.config.module_linking_enabled)
            .wasm_multi_memory(self.module_config.config.max_memories > 1)
            .wasm_simd(self.module_config.config.simd_enabled)
            .wasm_memory64(self.module_config.config.memory64_enabled)
            .cranelift_nan_canonicalization(self.wasmtime.canonicalize_nans)
            .cranelift_opt_level(self.wasmtime.opt_level.to_wasmtime())
            .interruptable(self.wasmtime.interruptable)
            .consume_fuel(self.wasmtime.consume_fuel)
            .memfd(self.wasmtime.memfd);

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
                cfg.cranelift_flag_set("wasmtime_linkopt_force_jump_veneer", "true")
                    .unwrap();
            }
        }

        match &self.wasmtime.memory_config {
            MemoryConfig::Normal {
                static_memory_maximum_size,
                static_memory_guard_size,
                dynamic_memory_guard_size,
                guard_before_linear_memory,
            } => {
                cfg.static_memory_maximum_size(static_memory_maximum_size.unwrap_or(0).into())
                    .static_memory_guard_size(static_memory_guard_size.unwrap_or(0).into())
                    .dynamic_memory_guard_size(dynamic_memory_guard_size.unwrap_or(0).into())
                    .guard_before_linear_memory(*guard_before_linear_memory);
            }
            MemoryConfig::CustomUnaligned => {
                cfg.with_host_memory(Arc::new(UnalignedMemoryCreator))
                    .static_memory_maximum_size(0)
                    .dynamic_memory_guard_size(0)
                    .static_memory_guard_size(0)
                    .guard_before_linear_memory(false);
            }
        }
        return cfg;
    }

    /// Convenience function for generating a `Store<T>` using this
    /// configuration.
    pub fn to_store(&self) -> Store<StoreLimits> {
        let engine = Engine::new(&self.to_wasmtime()).unwrap();
        let mut store = Store::new(&engine, StoreLimits::new());
        store.limiter(|s| s as &mut dyn wasmtime::ResourceLimiter);
        if self.wasmtime.consume_fuel {
            store.add_fuel(u64::max_value()).unwrap();
        }
        return store;
    }

    /// Generates an arbitrary method of timing out an instance, ensuring that
    /// this configuration supports the returned timeout.
    pub fn generate_timeout(&mut self, u: &mut Unstructured<'_>) -> arbitrary::Result<Timeout> {
        if u.arbitrary()? {
            self.wasmtime.interruptable = true;
            Ok(Timeout::Time(Duration::from_secs(20)))
        } else {
            self.wasmtime.consume_fuel = true;
            Ok(Timeout::Fuel(100_000))
        }
    }
}

struct UnalignedMemoryCreator;

unsafe impl MemoryCreator for UnalignedMemoryCreator {
    fn new_memory(
        &self,
        _ty: MemoryType,
        minimum: usize,
        maximum: Option<usize>,
        reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String> {
        assert_eq!(guard_size_in_bytes, 0);
        assert!(reserved_size_in_bytes.is_none() || reserved_size_in_bytes == Some(0));
        Ok(Box::new(UnalignedMemory {
            src: vec![0; minimum + 1],
            maximum,
        }))
    }
}

/// A custom "linear memory allocator" for wasm which only works with the
/// "dynamic" mode of configuration where wasm always does explicit bounds
/// checks.
///
/// This memory attempts to always use unaligned host addresses for the base
/// address of linear memory with wasm. This means that all jit loads/stores
/// should be unaligned, which is a "big hammer way" of testing that all our JIT
/// code works with unaligned addresses since alignment is not required for
/// correctness in wasm itself.
struct UnalignedMemory {
    /// This memory is always one byte larger than the actual size of linear
    /// memory.
    src: Vec<u8>,
    maximum: Option<usize>,
}

unsafe impl LinearMemory for UnalignedMemory {
    fn byte_size(&self) -> usize {
        // Chop off the extra byte reserved for the true byte size of this
        // linear memory.
        self.src.len() - 1
    }

    fn maximum_byte_size(&self) -> Option<usize> {
        self.maximum
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        // Make sure to allocate an extra byte for our "unalignment"
        self.src.resize(new_size + 1, 0);
        Ok(())
    }

    fn as_ptr(&self) -> *mut u8 {
        // Return our allocated memory, offset by one, so that the base address
        // of memory is always unaligned.
        self.src[1..].as_ptr() as *mut _
    }
}

include!(concat!(env!("OUT_DIR"), "/spectests.rs"));

/// A spec test from the upstream wast testsuite, arbitrarily chosen from the
/// list of known spec tests.
#[derive(Debug)]
pub struct SpecTest {
    /// The filename of the spec test
    pub file: &'static str,
    /// The `*.wast` contents of the spec test
    pub contents: &'static str,
}

impl<'a> Arbitrary<'a> for SpecTest {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // NB: this does get a uniform value in the provided range.
        let i = u.int_in_range(0..=FILES.len() - 1)?;
        let (file, contents) = FILES[i];
        Ok(SpecTest { file, contents })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(std::mem::size_of::<usize>()))
    }
}

/// Default module-level configuration for fuzzing Wasmtime.
///
/// Internally this uses `wasm-smith`'s own `SwarmConfig` but we further refine
/// the defaults here as well.
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    #[allow(missing_docs)]
    pub config: SwarmConfig,
}

impl ModuleConfig {
    /// Uses this configuration and the supplied source of data to generate
    /// a wasm module.
    pub fn generate(&self, input: &mut Unstructured<'_>) -> arbitrary::Result<wasm_smith::Module> {
        wasm_smith::Module::new(self.config.clone(), input)
    }

    /// Indicates that this configuration should be spec-test-compliant,
    /// disabling various features the spec tests assert are disabled.
    pub fn set_spectest_compliant(&mut self) {
        self.config.memory64_enabled = false;
        self.config.simd_enabled = false;
        self.config.bulk_memory_enabled = true;
        self.config.reference_types_enabled = true;
        self.config.max_memories = 1;
    }

    /// Indicates that this configuration is being used for differential
    /// execution so only a single function should be generated since that's all
    /// that's going to be exercised.
    pub fn set_differential_config(&mut self) {
        self.config.allow_start_export = false;
        // Make sure there's a type available for the function.
        self.config.min_types = 1;
        self.config.max_types = 1;

        // Generate one and only one function
        self.config.min_funcs = 1;
        self.config.max_funcs = 1;

        // Give the function a memory, but keep it small
        self.config.min_memories = 1;
        self.config.max_memories = 1;
        self.config.max_memory_pages = 1;
        self.config.memory_max_size_required = true;

        // Don't allow any imports
        self.config.max_imports = 0;

        // Try to get the function and the memory exported
        self.config.min_exports = 2;
        self.config.max_exports = 4;

        // NaN is canonicalized at the wasm level for differential fuzzing so we
        // can paper over NaN differences between engines.
        self.config.canonicalize_nans = true;

        // When diffing against a non-wasmtime engine then disable wasm
        // features to get selectively re-enabled against each differential
        // engine.
        self.config.bulk_memory_enabled = false;
        self.config.reference_types_enabled = false;
        self.config.simd_enabled = false;
        self.config.memory64_enabled = false;
    }
}

impl<'a> Arbitrary<'a> for ModuleConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<ModuleConfig> {
        let mut config = SwarmConfig::arbitrary(u)?;

        // Allow multi-memory by default.
        config.max_memories = config.max_memories.max(2);

        // Allow multi-table by default.
        config.max_tables = config.max_tables.max(4);

        // Allow enabling some various wasm proposals by default.
        config.bulk_memory_enabled = u.arbitrary()?;
        config.reference_types_enabled = u.arbitrary()?;
        config.simd_enabled = u.arbitrary()?;
        config.memory64_enabled = u.arbitrary()?;

        Ok(ModuleConfig { config })
    }
}
