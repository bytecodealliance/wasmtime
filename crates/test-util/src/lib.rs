#[cfg(feature = "component")]
pub mod component;
#[cfg(feature = "component-fuzz")]
pub mod component_fuzz;
#[cfg(feature = "wasmtime-wast")]
pub mod wasmtime_wast;
#[cfg(feature = "wast")]
pub mod wast;
