//! Define the interface for differential evaluation of Wasm functions.

use crate::generators::{DiffValue, ModuleFeatures};
use crate::oracles::{diff_wasmi::WasmiEngine, diff_wasmtime::WasmtimeEngine};
use arbitrary::Unstructured;
use std::collections::hash_map::DefaultHasher;

/// Pick one of the engines implemented in this module that is compatible with
/// the Wasm features passed in `features` and, when fuzzing Wasmtime against
/// itself, an existing `wasmtime_engine`.
pub fn choose(
    u: &mut Unstructured<'_>,
    features: &ModuleFeatures,
    wasmtime_engine: &WasmtimeEngine,
) -> arbitrary::Result<Box<dyn DiffEngine>> {
    // Filter out any engines that cannot match the given configuration.
    let mut engines: Vec<Box<dyn DiffEngine>> = vec![];
    if let Result::Ok(e) = WasmtimeEngine::arbitrary_with_compatible_config(u, wasmtime_engine) {
        engines.push(e)
    }
    if let Result::Ok(e) = WasmiEngine::new(features) {
        engines.push(e)
    }
    #[cfg(feature = "fuzz-spec-interpreter")]
    if let Result::Ok(e) = crate::oracles::diff_spec::SpecInterpreter::new(features) {
        engines.push(e)
    }

    // Choose one of the remaining engines.
    if !engines.is_empty() {
        let index: usize = u.int_in_range(0..=engines.len() - 1)?;
        Ok(engines.swap_remove(index))
    } else {
        panic!("no engines to pick from");
        // Err(arbitrary::Error::EmptyChoose)
    }
}

/// Provide a way to instantiate Wasm modules.
pub trait DiffEngine {
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
    fn hash(&self, state: &mut DefaultHasher) -> anyhow::Result<()>;
}

/// For errors that we want to ignore (not fuzz bugs), we can wrap them in this
/// structure.
#[derive(Debug)]
pub struct DiffIgnoreError(pub String);
impl std::fmt::Display for DiffIgnoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "this error should be ignored by fuzzing: {}", self.0)
    }
}
impl std::error::Error for DiffIgnoreError {}

/// This trait adds a handy way to ignore [`DiffIgnoreError`] during fuzzing.
pub trait DiffIgnorable<T> {
    /// Like `Result::expect`, but ignores all [`DiffIgnoreError`]s by logging
    /// the error and converting it to an `arbitrary` error.
    ///
    /// # Panics
    ///
    /// Panics if the value is an `Err` but not a [`DiffIgnoreError`].
    fn expect_or_ignore(self, message: &str) -> arbitrary::Result<T>;
}
impl<T> DiffIgnorable<T> for anyhow::Result<T> {
    fn expect_or_ignore(self, message: &str) -> arbitrary::Result<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                if let Some(ignorable) = e.downcast_ref::<DiffIgnoreError>() {
                    println!("ignoring error: {}", ignorable);
                    Err(arbitrary::Error::IncorrectFormat)
                } else {
                    panic!("{}: {:?}", message, e);
                }
            }
        }
    }
}

/// Extract the signatures of any exported functions in a Wasm module.
///
/// This is useful for evaluating each exported function with different values.
/// The [`DiffInstance`] trait asks for the function name and we need to know
/// the function signature in order to pass in the right arguments.
pub fn get_exported_function_signatures(
    wasm: &[u8],
) -> anyhow::Result<Vec<(String, wasmparser::FuncType)>> {
    let mut types = vec![];
    let mut functions_to_types = vec![];
    let mut signatures = vec![];
    for payload in wasmparser::Parser::new(0).parse_all(&wasm) {
        match payload? {
            wasmparser::Payload::TypeSection(s) => {
                for ty in s {
                    types.push(ty?);
                }
            }
            wasmparser::Payload::FunctionSection(s) => {
                for ty_index in s {
                    functions_to_types.push(ty_index?);
                }
            }
            wasmparser::Payload::ExportSection(s) => {
                for export in s {
                    let export = export?;
                    if export.kind == wasmparser::ExternalKind::Func {
                        let ty_index = functions_to_types[export.index as usize];
                        let ty = &types[ty_index as usize];
                        match ty {
                            wasmparser::Type::Func(ty) => {
                                signatures.push((export.name.to_string(), ty.clone()))
                            }
                        }
                    }
                }
            }
            _ => {
                // Ignore everything else.
            }
        }
    }
    Ok(signatures)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_exported_functions() {
        let wat = r#"(module
            (func (export "a1") (result i32) (i32.const 42))
            (global (export "b") (mut i32) (i32.const 42))
            (func (export "a2") (param i64) (result i32) (i32.const 42))
            (memory (export "c") 1 2 shared)
            (func (export "a3") (result i32) (i32.const 42))
        )"#;
        let wasm = wat::parse_str(wat).unwrap();
        let signatures = get_exported_function_signatures(&wasm).unwrap();
        let ty_odd = wasmparser::FuncType {
            params: vec![].into_boxed_slice(),
            returns: vec![wasmparser::ValType::I32].into_boxed_slice(),
        };
        let ty_even = wasmparser::FuncType {
            params: vec![wasmparser::ValType::I64].into_boxed_slice(),
            returns: vec![wasmparser::ValType::I32].into_boxed_slice(),
        };
        assert_eq!(
            signatures,
            vec![
                ("a1".to_string(), ty_odd.clone()),
                ("a2".to_string(), ty_even),
                ("a3".to_string(), ty_odd)
            ],
        );
    }
}
