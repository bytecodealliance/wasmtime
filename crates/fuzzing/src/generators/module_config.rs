//! Generate a configuration for generating a Wasm module.

use arbitrary::{Arbitrary, Unstructured};
use wasm_smith::SwarmConfig;

/// Default module-level configuration for fuzzing Wasmtime.
///
/// Internally this uses `wasm-smith`'s own `SwarmConfig` but we further refine
/// the defaults here as well.
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    #[allow(missing_docs)]
    pub config: SwarmConfig,
}

impl<'a> Arbitrary<'a> for ModuleConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<ModuleConfig> {
        let mut config = SwarmConfig::arbitrary(u)?;

        // Allow multi-memory by default.
        config.max_memories = config.max_memories.max(2);

        // Allow multi-table by default.
        config.max_tables = config.max_tables.max(4);

        // Allow enabling some various wasm proposals by default. Note that
        // these are all unconditionally turned off even with
        // `SwarmConfig::arbitrary`.
        config.memory64_enabled = u.arbitrary()?;

        // Allow the threads proposal if memory64 is not already enabled. FIXME:
        // to allow threads and memory64 to coexist, see
        // https://github.com/bytecodealliance/wasmtime/issues/4267.
        config.threads_enabled = !config.memory64_enabled && u.arbitrary()?;

        Ok(ModuleConfig { config })
    }
}
