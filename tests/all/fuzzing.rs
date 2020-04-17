//! Regression tests for bugs found via fuzzing.
//!
//! The `#[test]` goes in here, the Wasm binary goes in
//! `./fuzzing/some-descriptive-name.wasm`, and then the `#[test]` should
//! use the Wasm binary by including it via
//! `include_bytes!("./fuzzing/some-descriptive-name.wasm")`.

use wasmtime::{Config, Strategy};
use wasmtime_fuzzing::oracles;

#[test]
fn instantiate_empty_module() {
    let data = wat::parse_str(include_str!("./fuzzing/empty.wat")).unwrap();
    oracles::instantiate(&data, Strategy::Auto);
}

#[test]
fn instantiate_empty_module_with_memory() {
    let data = wat::parse_str(include_str!("./fuzzing/empty_with_memory.wat")).unwrap();
    oracles::instantiate(&data, Strategy::Auto);
}

#[test]
fn instantiate_module_that_compiled_to_x64_has_register_32() {
    let mut config = Config::new();
    config.debug_info(true);
    let data = wat::parse_str(include_str!("./fuzzing/issue694.wat")).unwrap();
    oracles::instantiate_with_config(&data, config);
}
