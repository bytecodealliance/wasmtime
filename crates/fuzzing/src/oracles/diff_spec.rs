//! Evaluate an exported Wasm function using the WebAssembly specification
//! reference interpreter.

use crate::generators::{Config, DiffValue, DiffValueType};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{anyhow, Error, Result};
use wasm_spec_interpreter::SpecValue;
use wasmtime::Trap;

/// A wrapper for `wasm-spec-interpreter` as a [`DiffEngine`].
pub struct SpecInterpreter;

impl SpecInterpreter {
    pub(crate) fn new(config: &mut Config) -> Self {
        let config = &mut config.module_config.config;

        config.min_memories = config.min_memories.min(1);
        config.max_memories = config.max_memories.min(1);
        config.min_tables = config.min_tables.min(1);
        config.max_tables = config.max_tables.min(1);

        config.memory64_enabled = false;
        config.threads_enabled = false;
        config.bulk_memory_enabled = false;
        config.reference_types_enabled = false;
        config.tail_call_enabled = false;
        config.relaxed_simd_enabled = false;
        config.custom_page_sizes_enabled = false;
        config.wide_arithmetic_enabled = false;
        config.extended_const_enabled = false;

        Self
    }
}

impl DiffEngine for SpecInterpreter {
    fn name(&self) -> &'static str {
        "spec"
    }

    fn instantiate(&mut self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let instance = wasm_spec_interpreter::instantiate(wasm)
            .map_err(|e| anyhow!("failed to instantiate in spec interpreter: {}", e))?;
        Ok(Box::new(SpecInstance { instance }))
    }

    fn assert_error_match(&self, trap: &Trap, err: &Error) {
        // TODO: implement this for the spec interpreter
        let _ = (trap, err);
    }

    fn is_stack_overflow(&self, err: &Error) -> bool {
        err.to_string().contains("(Isabelle) call stack exhausted")
    }
}

struct SpecInstance {
    instance: wasm_spec_interpreter::SpecInstance,
}

impl DiffInstance for SpecInstance {
    fn name(&self) -> &'static str {
        "spec"
    }

    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
        _results: &[DiffValueType],
    ) -> Result<Option<Vec<DiffValue>>> {
        let arguments = arguments.iter().map(SpecValue::from).collect();
        match wasm_spec_interpreter::interpret(&self.instance, function_name, Some(arguments)) {
            Ok(results) => Ok(Some(results.into_iter().map(SpecValue::into).collect())),
            Err(err) => Err(anyhow!(err)),
        }
    }

    fn get_global(&mut self, name: &str, _ty: DiffValueType) -> Option<DiffValue> {
        use wasm_spec_interpreter::{export, SpecExport::Global};
        if let Ok(Global(g)) = export(&self.instance, name) {
            Some(g.into())
        } else {
            panic!("expected an exported global value at name `{name}`")
        }
    }

    fn get_memory(&mut self, name: &str, _shared: bool) -> Option<Vec<u8>> {
        use wasm_spec_interpreter::{export, SpecExport::Memory};
        if let Ok(Memory(m)) = export(&self.instance, name) {
            Some(m)
        } else {
            panic!("expected an exported memory at name `{name}`")
        }
    }
}

impl From<&DiffValue> for SpecValue {
    fn from(v: &DiffValue) -> Self {
        match *v {
            DiffValue::I32(n) => SpecValue::I32(n),
            DiffValue::I64(n) => SpecValue::I64(n),
            DiffValue::F32(n) => SpecValue::F32(n as i32),
            DiffValue::F64(n) => SpecValue::F64(n as i64),
            DiffValue::V128(n) => SpecValue::V128(n.to_le_bytes().to_vec()),
            DiffValue::FuncRef { .. } | DiffValue::ExternRef { .. } | DiffValue::AnyRef { .. } => {
                unimplemented!()
            }
        }
    }
}

impl Into<DiffValue> for SpecValue {
    fn into(self) -> DiffValue {
        match self {
            SpecValue::I32(n) => DiffValue::I32(n),
            SpecValue::I64(n) => DiffValue::I64(n),
            SpecValue::F32(n) => DiffValue::F32(n as u32),
            SpecValue::F64(n) => DiffValue::F64(n as u64),
            SpecValue::V128(n) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        if !wasm_spec_interpreter::support_compiled_in() {
            return;
        }
        crate::oracles::engine::smoke_test_engine(|_, config| Ok(SpecInterpreter::new(config)))
    }
}
