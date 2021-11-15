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

use anyhow::Result;
use arbitrary::{Arbitrary, Unstructured};
use std::sync::Arc;
use wasmtime::{LinearMemory, MemoryCreator, MemoryType};

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

/// Implementation of generating a `wasmtime::Config` arbitrarily
#[derive(Arbitrary, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    opt_level: OptLevel,
    debug_info: bool,
    canonicalize_nans: bool,
    interruptable: bool,
    #[allow(missing_docs)]
    pub consume_fuel: bool,
    memory_config: MemoryConfig,
    force_jump_veneers: bool,
}

#[derive(Arbitrary, Debug, Eq, Hash, PartialEq)]
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
        let mut cfg = crate::fuzz_default_config(wasmtime::Strategy::Auto).unwrap();
        cfg.debug_info(self.debug_info)
            .cranelift_nan_canonicalization(self.canonicalize_nans)
            .cranelift_opt_level(self.opt_level.to_wasmtime())
            .interruptable(self.interruptable)
            .consume_fuel(self.consume_fuel);

        if self.force_jump_veneers {
            unsafe {
                cfg.cranelift_flag_set("wasmtime_linkopt_force_jump_veneer", "true")
                    .unwrap();
            }
        }

        match &self.memory_config {
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

/// Type alias for wasm-smith generated modules using wasmtime's default
/// configuration.
pub type GeneratedModule = wasm_smith::ConfiguredModule<WasmtimeDefaultConfig>;

/// Wasmtime-specific default configuration for wasm-smith-generated modules.
#[derive(Arbitrary, Clone, Debug)]
pub struct WasmtimeDefaultConfig;

impl wasm_smith::Config for WasmtimeDefaultConfig {
    // Allow multi-memory to get exercised
    fn max_memories(&self) -> usize {
        2
    }

    // Allow multi-table (reference types) to get exercised
    fn max_tables(&self) -> usize {
        4
    }

    fn reference_types_enabled(&self) -> bool {
        true
    }

    fn bulk_memory_enabled(&self) -> bool {
        true
    }

    fn memory64_enabled(&self) -> bool {
        true
    }
}
