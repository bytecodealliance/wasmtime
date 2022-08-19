//! Define the interface for differential evaluation of Wasm functions.

use crate::generators::{Config, DiffValue};
use crate::oracles::{diff_wasmi::WasmiEngine, diff_wasmtime::WasmtimeEngine};
use arbitrary::Unstructured;
use std::collections::hash_map::DefaultHasher;

/// Pick one of the engines implemented in this module that is compatible with
/// the Wasm features passed in `features` and, when fuzzing Wasmtime against
/// itself, an existing `wasmtime_engine`.
pub fn choose(
    u: &mut Unstructured<'_>,
    existing_config: &Config,
) -> arbitrary::Result<Box<dyn DiffEngine>> {
    // Filter out any engines that cannot match the given configuration.
    let mut engines: Vec<Box<dyn DiffEngine>> = vec![];
    let mut config: Config = u.arbitrary()?; // TODO change to WasmtimeConfig
    config.make_compatible_with(&existing_config);
    if let Result::Ok(e) = WasmtimeEngine::new(&config) {
        engines.push(e)
    }
    if let Result::Ok(e) = WasmiEngine::new(&existing_config.module_config) {
        engines.push(e)
    }
    #[cfg(feature = "fuzz-spec-interpreter")]
    if let Result::Ok(e) =
        crate::oracles::diff_spec::SpecInterpreter::new(&existing_config.module_config)
    {
        engines.push(e)
    }

    // Choose one of the remaining engines.
    if !engines.is_empty() {
        let index: usize = u.int_in_range(0..=engines.len() - 1)?;
        let engine = engines.swap_remove(index);
        log::debug!("selected engine: {}", engine.name());
        Ok(engine)
    } else {
        panic!("no engines to pick from");
        // Err(arbitrary::Error::EmptyChoose)
    }
}

/// Provide a way to instantiate Wasm modules.
pub trait DiffEngine {
    /// Return the name of the engine.
    fn name(&self) -> &'static str;

    /// Create a new instance with the given engine.
    fn instantiate(&self, wasm: &[u8]) -> anyhow::Result<Box<dyn DiffInstance>>;
}

/// Provide a way to evaluate Wasm functions--a Wasm instance implemented by a
/// specific engine (i.e., compiler or interpreter).
pub trait DiffInstance {
    /// Return the name of the engine behind this instance.
    fn name(&self) -> &'static str;

    /// Evaluate an exported function with the given values.
    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
    ) -> anyhow::Result<Vec<DiffValue>>;

    /// Check if instances of this kind are actually hashable--not all engines
    /// support this.
    fn is_hashable(&self) -> bool;

    /// If the instance `is_hashable()`, this method will try to hash the
    /// following exported items in the instance: globals, memory.
    ///
    /// TODO allow more types of hashers.
    fn hash(&mut self, state: &mut DefaultHasher) -> anyhow::Result<()>;
}
