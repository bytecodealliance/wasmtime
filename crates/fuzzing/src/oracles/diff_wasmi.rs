//! Evaluate an exported Wasm function using the wasmi interpreter.

use crate::generators::{self, DiffValue};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{bail, Context, Result};
use std::hash::Hash;
use wasm_smith::Config;

/// A wrapper for `wasmi` as a [`DiffEngine`].
pub struct WasmiEngine;

impl WasmiEngine {
    /// Build a new [`WasmiEngine`] but only if the configuration does not rely
    /// on features that `wasmi` does not support.
    pub fn new(config: &generators::Config) -> Result<Box<Self>> {
        let config = &config.module_config.config;
        if config.simd_enabled() {
            bail!("wasmi does not support SIMD")
        }
        if config.multi_value_enabled() {
            bail!("wasmi does not support multi-value")
        }
        if config.reference_types_enabled() {
            bail!("wasmi does not support reference types")
        }
        Ok(Box::new(Self))
    }
}

impl DiffEngine for WasmiEngine {
    fn instantiate(&self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let module =
            wasmi::Module::from_buffer(wasm).context("unable to validate module in wasmi")?;
        let instance = wasmi::ModuleInstance::new(&module, &wasmi::ImportsBuilder::default())
            .context("unable to instantiate module in wasmi")?;
        let instance = instance.assert_no_start();
        let exports = list_export_names(wasm);
        Ok(Box::new(WasmiInstance {
            module,
            exports,
            instance,
        }))
    }
}

/// A wrapper for `wasmi` Wasm instances.
struct WasmiInstance {
    #[allow(dead_code)] // reason = "the module must live as long as its reference"
    module: wasmi::Module,
    instance: wasmi::ModuleRef,
    /// `wasmi`'s instances have no way of listing their exports so, in order to
    /// properly hash the instance, we keep track of the export names.
    exports: Vec<String>,
}

impl DiffInstance for WasmiInstance {
    fn name(&self) -> &'static str {
        "wasmi"
    }

    fn evaluate(&mut self, function_name: &str, arguments: &[DiffValue]) -> Result<Vec<DiffValue>> {
        let arguments: Vec<_> = arguments.iter().map(wasmi::RuntimeValue::from).collect();
        let export = self
            .instance
            .export_by_name(function_name)
            .context(format!(
                "unable to find function '{}' in wasmi instance",
                function_name
            ))?;
        let function = export.as_func().context("wasmi export is not a function")?;
        let result = wasmi::FuncInstance::invoke(&function, &arguments, &mut wasmi::NopExternals)
            .context("failed while invoking function in wasmi")?;
        Ok(if let Some(result) = result {
            vec![result.into()]
        } else {
            vec![]
        })
    }

    fn is_hashable(&self) -> bool {
        true
    }

    fn hash(&self, state: &mut std::collections::hash_map::DefaultHasher) -> Result<()> {
        for export_name in &self.exports {
            if let Some(export) = self.instance.export_by_name(export_name) {
                match export {
                    wasmi::ExternVal::Func(_) => {}
                    wasmi::ExternVal::Table(_) => todo!(),
                    wasmi::ExternVal::Memory(m) => {
                        // `wasmi` memory may be stored non-contiguously; copy
                        // it out to a contiguous chunk.
                        let mut buffer: Vec<u8> = vec![0; m.current_size().0 * 65536];
                        m.get_into(0, &mut buffer[..])
                            .expect("can access wasmi memory");
                        buffer.hash(state)
                    }
                    wasmi::ExternVal::Global(g) => {
                        let val: DiffValue = g.get().into();
                        val.hash(state);
                    }
                }
            } else {
                panic!("unable to find export: {}", export_name)
            }
        }
        Ok(())
    }
}

/// List the names of all exported items in a binary Wasm module.
fn list_export_names(wasm: &[u8]) -> Vec<String> {
    let mut exports = vec![];
    for payload in wasmparser::Parser::new(0).parse_all(&wasm) {
        match payload.unwrap() {
            wasmparser::Payload::ExportSection(s) => {
                for export in s {
                    exports.push(export.unwrap().name.to_string());
                }
            }
            _ => {
                // Ignore any other sections.
            }
        }
    }
    exports
}

impl From<&DiffValue> for wasmi::RuntimeValue {
    fn from(v: &DiffValue) -> Self {
        use wasmi::RuntimeValue::*;
        match *v {
            DiffValue::I32(n) => I32(n),
            DiffValue::I64(n) => I64(n),
            DiffValue::F32(n) => F32(wasmi::nan_preserving_float::F32::from_bits(n)),
            DiffValue::F64(n) => F64(wasmi::nan_preserving_float::F64::from_bits(n)),
            DiffValue::V128(_) => unimplemented!(),
        }
    }
}

impl Into<DiffValue> for wasmi::RuntimeValue {
    fn into(self) -> DiffValue {
        use wasmi::RuntimeValue::*;
        match self {
            I32(n) => DiffValue::I32(n),
            I64(n) => DiffValue::I64(n),
            F32(n) => DiffValue::F32(n.to_bits()),
            F64(n) => DiffValue::F64(n.to_bits()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_export_names() {
        let wat = r#"(module
            (func (export "a") (result i32) (i32.const 42))
            (global (export "b") (mut i32) (i32.const 42))
            (memory (export "c") 1 2 shared)
        )"#;
        let wasm = wat::parse_str(wat).unwrap();
        assert_eq!(
            list_export_names(&wasm),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
    }
}
