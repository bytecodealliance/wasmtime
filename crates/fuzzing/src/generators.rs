//! Test case generators.
//!
//! Test case generators take raw, unstructured input from a fuzzer
//! (e.g. libFuzzer) and translate that into a structured test case (e.g. a
//! valid Wasm binary).
//!
//! These are generally implementations of the `Arbitrary` trait, or some
//! wrapper over an external tool, such that the wrapper implements the
//! `Arbitrary` trait for the wrapped external tool.

#[cfg(feature = "binaryen")]
pub mod api;

use arbitrary::{Arbitrary, Unstructured};

/// A Wasm test case generator that is powered by Binaryen's `wasm-opt -ttf`.
#[derive(Clone)]
#[cfg(feature = "binaryen")]
pub struct WasmOptTtf {
    /// The raw, encoded Wasm bytes.
    pub wasm: Vec<u8>,
}

#[cfg(feature = "binaryen")]
impl std::fmt::Debug for WasmOptTtf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "WasmOptTtf {{ wasm: wat::parse_str(r###\"\n{}\n\"###).unwrap() }}",
            wasmprinter::print_bytes(&self.wasm).expect("valid wasm should always disassemble")
        )
    }
}

#[cfg(feature = "binaryen")]
impl Arbitrary for WasmOptTtf {
    fn arbitrary(input: &mut arbitrary::Unstructured) -> arbitrary::Result<Self> {
        crate::init_fuzzing();
        let seed: Vec<u8> = Arbitrary::arbitrary(input)?;
        let module = binaryen::tools::translate_to_fuzz_mvp(&seed);
        let wasm = module.write();
        Ok(WasmOptTtf { wasm })
    }

    fn arbitrary_take_rest(input: arbitrary::Unstructured) -> arbitrary::Result<Self> {
        crate::init_fuzzing();
        let seed: Vec<u8> = Arbitrary::arbitrary_take_rest(input)?;
        let module = binaryen::tools::translate_to_fuzz_mvp(&seed);
        let wasm = module.write();
        Ok(WasmOptTtf { wasm })
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <Vec<u8> as Arbitrary>::size_hint(depth)
    }
}

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

    // Note that we use 32-bit values here to avoid blowing the 64-bit address
    // space by requesting ungodly-large sizes/guards.
    static_memory_maximum_size: Option<u32>,
    static_memory_guard_size: Option<u32>,
    dynamic_memory_guard_size: Option<u32>,
}

impl Config {
    /// Converts this to a `wasmtime::Config` object
    pub fn to_wasmtime(&self) -> wasmtime::Config {
        let mut cfg = wasmtime::Config::new();
        cfg.debug_info(self.debug_info)
            .static_memory_maximum_size(self.static_memory_maximum_size.unwrap_or(0).into())
            .static_memory_guard_size(self.static_memory_guard_size.unwrap_or(0).into())
            .dynamic_memory_guard_size(self.dynamic_memory_guard_size.unwrap_or(0).into())
            .cranelift_nan_canonicalization(self.canonicalize_nans)
            .cranelift_opt_level(self.opt_level.to_wasmtime())
            .interruptable(self.interruptable);
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

impl Arbitrary for SpecTest {
    fn arbitrary(u: &mut Unstructured) -> arbitrary::Result<Self> {
        // NB: this does get a uniform value in the provided range.
        let i = u.int_in_range(0..=FILES.len() - 1)?;
        let (file, contents) = FILES[i];
        Ok(SpecTest { file, contents })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(std::mem::size_of::<usize>()))
    }
}
