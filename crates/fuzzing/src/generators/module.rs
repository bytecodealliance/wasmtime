//! Generate a Wasm module and the configuration for generating it.

use arbitrary::{Arbitrary, Unstructured};

/// Default module-level configuration for fuzzing Wasmtime.
///
/// Internally this uses `wasm-smith`'s own `Config` but we further refine
/// the defaults here as well.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "self-describing fields")]
pub struct ModuleConfig {
    pub config: wasm_smith::Config,

    // These knobs aren't exposed in `wasm-smith` at this time but are exposed
    // in our `*.wast` testing so keep knobs here so they can be read during
    // config-to-`wasmtime::Config` translation.
    pub function_references_enabled: bool,
    pub component_model_more_flags: bool,
    pub component_model_async: bool,
}

impl<'a> Arbitrary<'a> for ModuleConfig {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<ModuleConfig> {
        let mut config = wasm_smith::Config::arbitrary(u)?;

        // This list is intended to be the definintive source of truth for
        // what's at least possible to fuzz within Wasmtime. This is a
        // combination of features in `wasm-smith` where some proposals are
        // on-by-default (as determined by fuzz input) and others are
        // off-by-default (as they aren't stage4+). Wasmtime will default-fuzz
        // proposals that a pre-stage-4 to test our own implementation. Wasmtime
        // might also unconditionally disable proposals that it doesn't
        // implement yet which are stage4+. This is intended to be an exhaustive
        // list of all the wasm proposals that `wasm-smith` supports and the
        // fuzzing status within Wasmtime too.
        let _ = config.multi_value_enabled;
        let _ = config.saturating_float_to_int_enabled;
        let _ = config.sign_extension_ops_enabled;
        let _ = config.bulk_memory_enabled;
        let _ = config.reference_types_enabled;
        let _ = config.simd_enabled;
        let _ = config.relaxed_simd_enabled;
        let _ = config.tail_call_enabled;
        let _ = config.extended_const_enabled;
        config.exceptions_enabled = false;
        config.gc_enabled = false;
        config.custom_page_sizes_enabled = u.arbitrary()?;
        config.wide_arithmetic_enabled = u.arbitrary()?;
        config.memory64_enabled = u.ratio(1, 20)?;
        config.threads_enabled = u.ratio(1, 20)?;
        // Allow multi-memory but make it unlikely
        if u.ratio(1, 20)? {
            config.max_memories = config.max_memories.max(2);
        } else {
            config.max_memories = 1;
        }
        // ... NB: if you add something above this line please be sure to update
        // `docs/stability-wasm-proposals.md`

        // We get better differential execution when we disallow traps, so we'll
        // do that most of the time.
        config.disallow_traps = u.ratio(9, 10)?;

        Ok(ModuleConfig {
            component_model_more_flags: false,
            component_model_async: false,
            function_references_enabled: config.gc_enabled,
            config,
        })
    }
}

impl ModuleConfig {
    /// Uses this configuration and the supplied source of data to generate a
    /// Wasm module.
    ///
    /// If a `default_fuel` is provided, the resulting module will be configured
    /// to ensure termination; as doing so will add an additional global to the
    /// module, the pooling allocator, if configured, must also have its globals
    /// limit updated.
    pub fn generate(
        &self,
        input: &mut Unstructured<'_>,
        default_fuel: Option<u32>,
    ) -> arbitrary::Result<wasm_smith::Module> {
        let mut module = wasm_smith::Module::new(self.config.clone(), input)?;

        if let Some(default_fuel) = default_fuel {
            module.ensure_termination(default_fuel).unwrap();
        }

        Ok(module)
    }
}
