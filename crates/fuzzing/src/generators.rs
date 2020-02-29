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

use arbitrary::{Arbitrary, Unstructured};
use std::fmt;

/// A Wasm test case generator that is powered by Binaryen's `wasm-opt -ttf`.
#[derive(Clone)]
pub struct WasmOptTtf {
    /// The raw, encoded Wasm bytes.
    pub wasm: Vec<u8>,
}

impl fmt::Debug for WasmOptTtf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "WasmOptTtf {{ wasm: wat::parse_str(r###\"\n{}\n\"###).unwrap() }}",
            wasmprinter::print_bytes(&self.wasm).expect("valid wasm should always disassemble")
        )
    }
}

impl Arbitrary for WasmOptTtf {
    fn arbitrary(input: &mut Unstructured) -> arbitrary::Result<Self> {
        let seed: Vec<u8> = Arbitrary::arbitrary(input)?;
        let module = binaryen::tools::translate_to_fuzz_mvp(&seed);
        let wasm = module.write();
        Ok(WasmOptTtf { wasm })
    }

    fn arbitrary_take_rest(input: Unstructured) -> arbitrary::Result<Self> {
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
    opt_level: DifferentialOptLevel,
}

impl DifferentialConfig {
    /// Convert this differential fuzzing config into a `wasmtime::Config`.
    pub fn to_wasmtime_config(&self) -> anyhow::Result<wasmtime::Config> {
        let mut config = wasmtime::Config::new();
        config.strategy(match self.strategy {
            DifferentialStrategy::Cranelift => wasmtime::Strategy::Cranelift,
            DifferentialStrategy::Lightbeam => wasmtime::Strategy::Lightbeam,
        })?;
        config.cranelift_opt_level(match self.opt_level {
            DifferentialOptLevel::None => wasmtime::OptLevel::None,
            DifferentialOptLevel::Speed => wasmtime::OptLevel::Speed,
            DifferentialOptLevel::SpeedAndSize => wasmtime::OptLevel::SpeedAndSize,
        });
        Ok(config)
    }
}

#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
enum DifferentialStrategy {
    Cranelift,
    Lightbeam,
}

#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
enum DifferentialOptLevel {
    None,
    Speed,
    SpeedAndSize,
}
