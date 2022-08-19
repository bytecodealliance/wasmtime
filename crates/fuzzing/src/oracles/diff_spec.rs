//! Evaluate an exported Wasm function using the WebAssembly specification
//! reference interpreter.

use crate::generators::{DiffValue, ModuleConfig};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{anyhow, bail, Result};
use wasm_spec_interpreter::Value;

/// A wrapper for `wasm-spec-interpreter` as a [`DiffEngine`].
pub struct SpecInterpreter;

impl SpecInterpreter {
    /// Build a new [`SpecInterpreter`] but only if the configuration does not
    /// rely on features that the current bindings (i.e.,
    /// `wasm-spec-interpreter`) do not support.
    pub fn new(config: &ModuleConfig) -> Result<Box<Self>> {
        if config.config.reference_types_enabled {
            bail!("the spec interpreter bindings do not support reference types")
        }
        if config.config.max_funcs > 1 {
            // TODO
            bail!("the spec interpreter bindings can only support one function for now")
        }
        if config.config.max_tables > 0 {
            // TODO
            bail!("the spec interpreter bindings do not fail as they should with out-of-bounds table accesses")
        }
        Ok(Box::new(Self))
    }
}

impl DiffEngine for SpecInterpreter {
    fn name(&self) -> &'static str {
        "spec"
    }

    fn instantiate(&self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        // TODO: ideally we would avoid copying the module bytes here.
        Ok(Box::new(SpecInstance {
            wasm: wasm.to_vec(),
        }))
    }
}

struct SpecInstance {
    wasm: Vec<u8>,
}

impl DiffInstance for SpecInstance {
    fn name(&self) -> &'static str {
        "spec"
    }

    fn evaluate(
        &mut self,
        _function_name: &str,
        arguments: &[DiffValue],
    ) -> Result<Vec<DiffValue>> {
        // The spec interpreter needs some work before it can fully support this
        // interface:
        //  - TODO adapt `wasm-spec-interpreter` to use function name to select
        //    function to run
        //  - TODO adapt `wasm-spec-interpreter` to expose an "instance" with
        //    so we can hash memory, globals, etc.
        let arguments = arguments.iter().map(Value::from).collect();
        match wasm_spec_interpreter::interpret(&self.wasm, Some(arguments)) {
            Ok(results) => Ok(results.into_iter().map(Value::into).collect()),
            Err(err) => Err(anyhow!(err)),
        }
    }

    fn is_hashable(&self) -> bool {
        false
    }

    fn hash(&mut self, _state: &mut std::collections::hash_map::DefaultHasher) -> Result<()> {
        unimplemented!()
    }
}

impl From<&DiffValue> for Value {
    fn from(v: &DiffValue) -> Self {
        match *v {
            DiffValue::I32(n) => Value::I32(n),
            DiffValue::I64(n) => Value::I64(n),
            DiffValue::F32(n) => Value::F32(n as i32),
            DiffValue::F64(n) => Value::F64(n as i64),
            DiffValue::V128(n) => Value::V128(n.to_le_bytes().to_vec()),
        }
    }
}

impl Into<DiffValue> for Value {
    fn into(self) -> DiffValue {
        match self {
            Value::I32(n) => DiffValue::I32(n),
            Value::I64(n) => DiffValue::I64(n),
            Value::F32(n) => DiffValue::F32(n as u32),
            Value::F64(n) => DiffValue::F64(n as u64),
            Value::V128(n) => {
                assert_eq!(n.len(), 16);
                DiffValue::V128(u128::from_le_bytes(n.as_slice().try_into().unwrap()))
            }
        }
    }
}

/// Set up the OCaml runtime for triggering its signal handler configuration.
///
/// Because both the OCaml runtime and Wasmtime set up signal handlers, we must
/// carefully decide when to instantiate them; this function allows us to
/// control when. Wasmtime uses these signal handlers for catching various
/// WebAssembly failures. On certain OSes (e.g. Linux `x86_64`), the signal
/// handlers interfere, observable as an uncaught `SIGSEGV`--not even caught by
/// libFuzzer.
///
/// This failure can be mitigated by always running Wasmtime second in
/// differential fuzzing. In some cases, however, this is not possible because
/// which engine will execute first is unknown. This function can be explicitly
/// executed first, e.g., during global initialization, to avoid this issue.
pub fn setup_ocaml_runtime() {
    wasm_spec_interpreter::setup_ocaml_runtime();
}
