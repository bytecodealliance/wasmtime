//! Define the interface for differential evaluation of Wasm functions.

use crate::generators::{CompilerStrategy, Config, DiffValue, DiffValueType};
use crate::oracles::{diff_wasmi::WasmiEngine, diff_wasmtime::WasmtimeEngine};
use anyhow::Error;
use arbitrary::Unstructured;
use wasmtime::Trap;

/// Returns a function which can be used to build the engine name specified.
///
/// `None` is returned if the named engine does not have support compiled into
/// this crate.
pub fn build(
    u: &mut Unstructured<'_>,
    name: &str,
    config: &mut Config,
) -> arbitrary::Result<Option<Box<dyn DiffEngine>>> {
    let engine: Box<dyn DiffEngine> = match name {
        "wasmtime" => Box::new(WasmtimeEngine::new(u, config, CompilerStrategy::Cranelift)?),
        "wasmi" => Box::new(WasmiEngine::new(config)),

        #[cfg(target_arch = "x86_64")]
        "winch" => Box::new(WasmtimeEngine::new(u, config, CompilerStrategy::Winch)?),
        #[cfg(not(target_arch = "x86_64"))]
        "winch" => return Ok(None),

        #[cfg(feature = "fuzz-spec-interpreter")]
        "spec" => Box::new(crate::oracles::diff_spec::SpecInterpreter::new(config)),
        #[cfg(not(feature = "fuzz-spec-interpreter"))]
        "spec" => return Ok(None),

        #[cfg(not(any(windows, target_arch = "s390x", target_arch = "riscv64")))]
        "v8" => Box::new(crate::oracles::diff_v8::V8Engine::new(config)),
        #[cfg(any(windows, target_arch = "s390x", target_arch = "riscv64"))]
        "v8" => return Ok(None),

        _ => panic!("unknown engine {name}"),
    };

    Ok(Some(engine))
}

/// Provide a way to instantiate Wasm modules.
pub trait DiffEngine {
    /// Return the name of the engine.
    fn name(&self) -> &'static str;

    /// Create a new instance with the given engine.
    fn instantiate(&mut self, wasm: &[u8]) -> anyhow::Result<Box<dyn DiffInstance>>;

    /// Tests that the wasmtime-originating `trap` matches the error this engine
    /// generated.
    fn assert_error_match(&self, trap: &Trap, err: &Error);

    /// Returns whether the error specified from this engine might be stack
    /// overflow.
    fn is_stack_overflow(&self, err: &Error) -> bool;
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

/// Build a list of allowed values from the given `defaults` using the
/// `env_list`.
///
/// ```
/// # use wasmtime_fuzzing::oracles::engine::build_allowed_env_list;
/// // Passing no `env_list` returns the defaults:
/// assert_eq!(build_allowed_env_list(None, &["a"]), vec!["a"]);
/// // We can build up a subset of the defaults:
/// assert_eq!(build_allowed_env_list(Some(vec!["b".to_string()]), &["a","b"]), vec!["b"]);
/// // Alternately we can subtract from the defaults:
/// assert_eq!(build_allowed_env_list(Some(vec!["-a".to_string()]), &["a","b"]), vec!["b"]);
/// ```
/// ```should_panic
/// # use wasmtime_fuzzing::oracles::engine::build_allowed_env_list;
/// // We are not allowed to mix set "addition" and "subtraction"; the following
/// // will panic:
/// build_allowed_env_list(Some(vec!["-a".to_string(), "b".to_string()]), &["a", "b"]);
/// ```
/// ```should_panic
/// # use wasmtime_fuzzing::oracles::engine::build_allowed_env_list;
/// // This will also panic if invalid values are used:
/// build_allowed_env_list(Some(vec!["c".to_string()]), &["a", "b"]);
/// ```
pub fn build_allowed_env_list<'a>(
    env_list: Option<Vec<String>>,
    defaults: &[&'a str],
) -> Vec<&'a str> {
    if let Some(configured) = &env_list {
        // Check that the names are either all additions or all subtractions.
        let subtract_from_defaults = configured.iter().all(|c| c.starts_with("-"));
        let add_from_defaults = configured.iter().all(|c| !c.starts_with("-"));
        let start = if subtract_from_defaults { 1 } else { 0 };
        if !subtract_from_defaults && !add_from_defaults {
            panic!(
                "all configured values must either subtract or add from defaults; found mixed values: {:?}",
                &env_list
            );
        }

        // Check that the configured names are valid ones.
        for c in configured {
            if !defaults.contains(&&c[start..]) {
                panic!(
                    "invalid environment configuration `{c}`; must be one of: {defaults:?}"
                );
            }
        }

        // Select only the allowed names.
        let mut allowed = Vec::with_capacity(defaults.len());
        for &d in defaults {
            let mentioned = configured.iter().any(|c| &c[start..] == d);
            if (add_from_defaults && mentioned) || (subtract_from_defaults && !mentioned) {
                allowed.push(d);
            }
        }
        allowed
    } else {
        defaults.to_vec()
    }
}

/// Retrieve a comma-delimited list of values from an environment variable.
pub fn parse_env_list(env_variable: &str) -> Option<Vec<String>> {
    std::env::var(env_variable)
        .ok()
        .map(|l| l.split(",").map(|s| s.to_owned()).collect())
}

#[cfg(test)]
pub fn smoke_test_engine<T>(
    mk_engine: impl Fn(&mut arbitrary::Unstructured<'_>, &mut Config) -> arbitrary::Result<T>,
) where
    T: DiffEngine,
{
    use rand::prelude::*;

    let mut rng = SmallRng::seed_from_u64(0);
    let mut buf = vec![0; 2048];
    let n = 100;
    for _ in 0..n {
        rng.fill_bytes(&mut buf);
        let mut u = Unstructured::new(&buf);
        let mut config = match u.arbitrary::<Config>() {
            Ok(config) => config,
            Err(_) => continue,
        };
        // This will ensure that wasmtime, which uses this configuration
        // settings, can guaranteed instantiate a module.
        config.set_differential_config();

        let mut engine = match mk_engine(&mut u, &mut config) {
            Ok(engine) => engine,
            Err(e) => {
                println!("skip {e:?}");
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
