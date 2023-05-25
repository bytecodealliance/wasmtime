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
mod memory;
mod module;
mod pooling_config;
mod single_inst_module;
mod spec_test;
mod stacks;
pub mod table_ops;
mod value;

pub use codegen_settings::CodegenSettings;
pub use config::CompilerStrategy;
pub use config::{Config, WasmtimeConfig};
pub use instance_allocation_strategy::InstanceAllocationStrategy;
pub use memory::{MemoryConfig, NormalMemoryConfig, UnalignedMemory, UnalignedMemoryCreator};
pub use module::ModuleConfig;
pub use pooling_config::PoolingAllocationConfig;
pub use single_inst_module::SingleInstModule;
pub use spec_test::SpecTest;
pub use stacks::Stacks;
pub use value::{DiffValue, DiffValueType};
