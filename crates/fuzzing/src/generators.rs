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
mod codegen_settings;
mod config;
mod instance_allocation_strategy;
mod instance_limits;
mod memory;
mod module_config;
mod spec_test;
pub mod table_ops;

pub use codegen_settings::CodegenSettings;
pub use config::{Config, WasmtimeConfig};
pub use instance_allocation_strategy::InstanceAllocationStrategy;
pub use instance_limits::InstanceLimits;
pub use memory::{MemoryConfig, NormalMemoryConfig, UnalignedMemory, UnalignedMemoryCreator};
pub use module_config::ModuleConfig;
pub use spec_test::SpecTest;
