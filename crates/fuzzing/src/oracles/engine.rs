//! Define the interface for differential evaluation of Wasm functions.

use crate::generators::{Config, DiffValue, DiffValueType, WasmtimeConfig};
use crate::oracles::{diff_wasmi::WasmiEngine, diff_wasmtime::WasmtimeEngine};
use anyhow::Error;
use arbitrary::Unstructured;
use wasmtime::Trap;

/// Pick one of the engines implemented in this module that is compatible with
/// the Wasm features passed in `features` and, when fuzzing Wasmtime against
/// itself, an existing `wasmtime_engine`.
pub fn choose(
    u: &mut Unstructured<'_>,
    existing_config: &Config,
) -> arbitrary::Result<Box<dyn DiffEngine>> {
    // Filter out any engines that cannot match the given configuration.
    let mut engines: Vec<Box<dyn DiffEngine>> = vec![];
    let mut config2: WasmtimeConfig = u.arbitrary()?; // TODO change to WasmtimeConfig
    config2.make_compatible_with(&existing_config.wasmtime);
    let config2 = Config {
        wasmtime: config2,
        module_config: existing_config.module_config.clone(),
    };
    if let Result::Ok(e) = WasmtimeEngine::new(config2) {
        engines.push(Box::new(e))
    }
    if let Result::Ok(e) = WasmiEngine::new(&existing_config.module_config) {
        engines.push(Box::new(e))
    }
    #[cfg(feature = "fuzz-spec-interpreter")]
    if let Result::Ok(e) =
        crate::oracles::diff_spec::SpecInterpreter::new(&existing_config.module_config)
    {
        engines.push(Box::new(e))
    }
    #[cfg(not(any(windows, target_arch = "s390x")))]
    if let Result::Ok(e) = crate::oracles::diff_v8::V8Engine::new(&existing_config.module_config) {
        engines.push(Box::new(e))
    }

    // Use the input of the fuzzer to pick an engine that we'll be fuzzing
    // Wasmtime against.
    assert!(!engines.is_empty());
    let index: usize = u.int_in_range(0..=engines.len() - 1)?;
    let engine = engines.swap_remove(index);
    log::debug!("selected engine: {}", engine.name());
    Ok(engine)
}

/// Provide a way to instantiate Wasm modules.
pub trait DiffEngine {
    /// Return the name of the engine.
    fn name(&self) -> &'static str;

    /// Create a new instance with the given engine.
    fn instantiate(&mut self, wasm: &[u8]) -> anyhow::Result<Box<dyn DiffInstance>>;

    /// Tests that the wasmtime-originating `trap` matches the error this engine
    /// generated.
    fn assert_error_match(&self, trap: &Trap, err: Error);
}

/// Provide a way to evaluate Wasm functions--a Wasm instance implemented by a
/// specific engine (i.e., compiler or interpreter).
pub trait DiffInstance {
    /// Return the name of the engine behind this instance.
    fn name(&self) -> &'static str;

    /// Evaluate an exported function with the given values.
    ///
    /// Any error, such as a trap, should be returned through an `Err`. If this
    /// engine cannot invoke the function signature then `None` should be
    /// returned and this invocation will be skipped.
    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
        results: &[DiffValueType],
    ) -> anyhow::Result<Option<Vec<DiffValue>>>;

    /// Attempts to return the value of the specified global, returning `None`
    /// if this engine doesn't support retrieving globals at this time.
    fn get_global(&mut self, name: &str, ty: DiffValueType) -> Option<DiffValue>;

    /// Same as `get_global` but for memory.
    fn get_memory(&mut self, name: &str, shared: bool) -> Option<Vec<u8>>;
}

/// Initialize any global state associated with runtimes that may be
/// differentially executed against.
pub fn setup_engine_runtimes() {
    #[cfg(feature = "fuzz-spec-interpreter")]
    crate::oracles::diff_spec::setup_ocaml_runtime();
}

#[cfg(test)]
pub fn smoke_test_engine<T>(mk_engine: impl Fn(Config) -> anyhow::Result<T>)
where
    T: DiffEngine,
{
    use arbitrary::Arbitrary;
    use rand::prelude::*;

    let mut rng = SmallRng::seed_from_u64(0);
    let mut buf = vec![0; 2048];
    let n = 100;
    for _ in 0..n {
        rng.fill_bytes(&mut buf);
        let u = Unstructured::new(&buf);
        let mut config = match Config::arbitrary_take_rest(u) {
            Ok(config) => config,
            Err(_) => continue,
        };
        // This will ensure that wasmtime, which uses this configuration
        // settings, can guaranteed instantiate a module.
        config.set_differential_config();

        // Configure settings to ensure that any filters in engine constructors
        // try not to filter out this `Config`.
        config.module_config.config.reference_types_enabled = false;
        config.module_config.config.bulk_memory_enabled = false;
        config.module_config.config.memory64_enabled = false;
        config.module_config.config.threads_enabled = false;
        config.module_config.config.simd_enabled = false;
        config.module_config.config.min_funcs = 1;
        config.module_config.config.max_funcs = 1;
        config.module_config.config.min_tables = 0;
        config.module_config.config.max_tables = 0;

        let mut engine = match mk_engine(config) {
            Ok(engine) => engine,
            Err(e) => {
                println!("skip {:?}", e);
                continue;
            }
        };

        let wasm = wat::parse_str(
            r#"
                (module
                    (func (export "add") (param i32 i32) (result i32)
                        local.get 0
                        local.get 1
                        i32.add)

                    (global (export "global") i32 i32.const 1)
                    (memory (export "memory") 1)
                )
            "#,
        )
        .unwrap();
        let mut instance = engine.instantiate(&wasm).unwrap();
        let results = instance
            .evaluate(
                "add",
                &[DiffValue::I32(1), DiffValue::I32(2)],
                &[DiffValueType::I32],
            )
            .unwrap();
        assert_eq!(results, Some(vec![DiffValue::I32(3)]));

        if let Some(val) = instance.get_global("global", DiffValueType::I32) {
            assert_eq!(val, DiffValue::I32(1));
        }

        if let Some(val) = instance.get_memory("memory", false) {
            assert_eq!(val.len(), 65536);
            for i in val.iter() {
                assert_eq!(*i, 0);
            }
        }

        return;
    }

    panic!("after {n} runs nothing ever ran, something is probably wrong");
}
