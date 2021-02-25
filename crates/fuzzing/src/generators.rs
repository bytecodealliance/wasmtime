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

use arbitrary::{Arbitrary, Unstructured};

/// A description of configuration options that we should do differential
/// testing between.
#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DifferentialConfig {
    strategy: DifferentialStrategy,
    opt_level: OptLevel,
}

impl DifferentialConfig {
    /// Convert this differential fuzzing config into a `wasmtime::Config`.
    pub fn to_wasmtime_config(&self) -> anyhow::Result<wasmtime::Config> {
        let mut config = crate::fuzz_default_config(match self.strategy {
            DifferentialStrategy::Cranelift => wasmtime::Strategy::Cranelift,
            DifferentialStrategy::Lightbeam => wasmtime::Strategy::Lightbeam,
        })?;
        config.cranelift_opt_level(self.opt_level.to_wasmtime());
        Ok(config)
    }
}

#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
enum DifferentialStrategy {
    Cranelift,
    Lightbeam,
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

/// Implementation of generating a `wasmtime::Config` arbitrarily
#[derive(Arbitrary, Debug)]
pub struct Config {
    opt_level: OptLevel,
    debug_info: bool,
    canonicalize_nans: bool,
    interruptable: bool,
    #[allow(missing_docs)]
    pub consume_fuel: bool,

    // Note that we use 32-bit values here to avoid blowing the 64-bit address
    // space by requesting ungodly-large sizes/guards.
    static_memory_maximum_size: Option<u32>,
    static_memory_guard_size: Option<u32>,
    dynamic_memory_guard_size: Option<u32>,
}

impl Config {
    /// Converts this to a `wasmtime::Config` object
    pub fn to_wasmtime(&self) -> wasmtime::Config {
        let mut cfg = crate::fuzz_default_config(wasmtime::Strategy::Auto).unwrap();
        cfg.debug_info(self.debug_info)
            .static_memory_maximum_size(self.static_memory_maximum_size.unwrap_or(0).into())
            .static_memory_guard_size(self.static_memory_guard_size.unwrap_or(0).into())
            .dynamic_memory_guard_size(self.dynamic_memory_guard_size.unwrap_or(0).into())
            .cranelift_nan_canonicalization(self.canonicalize_nans)
            .cranelift_opt_level(self.opt_level.to_wasmtime())
            .interruptable(self.interruptable)
            .consume_fuel(self.consume_fuel);
        return cfg;
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
