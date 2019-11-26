//! Regression tests for bugs found via fuzzing.
//!
//! The `#[test]` goes in here, the Wasm binary goes in
//! `./regressions/some-descriptive-name.wasm`, and then the `#[test]` should
//! use the Wasm binary by including it via
//! `include_bytes!("./regressions/some-descriptive-name.wasm")`.

#[allow(unused_imports)] // Until we actually have some regression tests...
use wasmtime_fuzzing::*;
