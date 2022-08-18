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
pub mod component_types;
mod config;
mod instance_allocation_strategy;
mod instance_limits;
mod memory;
mod module;
mod single_inst_module;
mod spec_test;
mod stacks;
pub mod table_ops;
mod value;

pub use codegen_settings::CodegenSettings;
pub use config::{Config, WasmtimeConfig};
pub use instance_allocation_strategy::InstanceAllocationStrategy;
pub use instance_limits::InstanceLimits;
pub use memory::{MemoryConfig, NormalMemoryConfig, UnalignedMemory, UnalignedMemoryCreator};
pub use module::ModuleConfig;
pub use single_inst_module::SingleInstModule;
pub use spec_test::SpecTest;
pub use stacks::Stacks;
pub use value::{DiffValue, DiffValueType};
