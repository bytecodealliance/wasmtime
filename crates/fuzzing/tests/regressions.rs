//! Regression tests for bugs found via fuzzing.
//!
//! The `#[test]` goes in here, the Wasm binary goes in
//! `./regressions/some-descriptive-name.wasm`, and then the `#[test]` should
//! use the Wasm binary by including it via
//! `include_bytes!("./regressions/some-descriptive-name.wasm")`.

use wasmtime::Strategy;
use wasmtime_fuzzing::oracles;

#[test]
fn instantiate_empty_module() {
    let data = wat::parse_str(include_str!("./regressions/empty.wat")).unwrap();
    oracles::instantiate(&data, Strategy::Auto);
}

#[test]
fn instantiate_empty_module_with_memory() {
    let data = wat::parse_str(include_str!("./regressions/empty_with_memory.wat")).unwrap();
    oracles::instantiate(&data, Strategy::Auto);
}
