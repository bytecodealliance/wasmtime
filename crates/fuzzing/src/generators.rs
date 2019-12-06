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
    fn arbitrary<U>(input: &mut U) -> Result<Self, U::Error>
    where
        U: Unstructured + ?Sized,
    {
        let seed: Vec<u8> = Arbitrary::arbitrary(input)?;
        let module = binaryen::tools::translate_to_fuzz_mvp(&seed);
        let wasm = module.write();
        Ok(WasmOptTtf { wasm })
    }
}
