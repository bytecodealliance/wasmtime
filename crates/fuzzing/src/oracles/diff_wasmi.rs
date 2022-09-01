//! Evaluate an exported Wasm function using the wasmi interpreter.

use crate::generators::{DiffValue, DiffValueType, ModuleConfig};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{bail, Context, Error, Result};
use wasmtime::Trap;

/// A wrapper for `wasmi` as a [`DiffEngine`].
pub struct WasmiEngine;

impl WasmiEngine {
    /// Build a new [`WasmiEngine`] but only if the configuration does not rely
    /// on features that `wasmi` does not support.
    pub fn new(config: &ModuleConfig) -> Result<Self> {
        if config.config.reference_types_enabled {
            bail!("wasmi does not support reference types")
        }
        if config.config.simd_enabled {
            bail!("wasmi does not support SIMD")
        }
        if config.config.multi_value_enabled {
            bail!("wasmi does not support multi-value")
        }
        if config.config.saturating_float_to_int_enabled {
            bail!("wasmi does not support saturating float-to-int conversions")
        }
        if config.config.sign_extension_enabled {
            bail!("wasmi does not support sign-extension")
        }
        if config.config.memory64_enabled {
            bail!("wasmi does not support memory64");
        }
        if config.config.bulk_memory_enabled {
            bail!("wasmi does not support bulk memory");
        }
        if config.config.threads_enabled {
            bail!("wasmi does not support threads");
        }
        Ok(Self)
    }
}

impl DiffEngine for WasmiEngine {
    fn name(&self) -> &'static str {
        "wasmi"
    }

    fn instantiate(&mut self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let module = wasmi::Module::from_buffer(wasm).context("unable to validate Wasm module")?;
        let instance = wasmi::ModuleInstance::new(&module, &wasmi::ImportsBuilder::default())
            .context("unable to instantiate module in wasmi")?;
        let instance = instance.assert_no_start();
        Ok(Box::new(WasmiInstance { module, instance }))
    }

    fn assert_error_match(&self, trap: &Trap, err: Error) {
        // TODO: should implement this for `wasmi`
        drop((trap, err));
    }
}

/// A wrapper for `wasmi` Wasm instances.
struct WasmiInstance {
    #[allow(dead_code)] // reason = "the module must live as long as its reference"
    module: wasmi::Module,
    instance: wasmi::ModuleRef,
}

impl DiffInstance for WasmiInstance {
    fn name(&self) -> &'static str {
        "wasmi"
    }

    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
        _results: &[DiffValueType],
    ) -> Result<Option<Vec<DiffValue>>> {
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
        Ok(Some(if let Some(result) = result {
            vec![result.into()]
        } else {
            vec![]
        }))
    }

    fn get_global(&mut self, name: &str, _ty: DiffValueType) -> Option<DiffValue> {
        match self.instance.export_by_name(name) {
            Some(wasmi::ExternVal::Global(g)) => Some(g.get().into()),
            _ => unreachable!(),
        }
    }

    fn get_memory(&mut self, name: &str, shared: bool) -> Option<Vec<u8>> {
        assert!(!shared);
        match self.instance.export_by_name(name) {
            Some(wasmi::ExternVal::Memory(m)) => {
                // `wasmi` memory may be stored non-contiguously; copy
                // it out to a contiguous chunk.
                let mut buffer: Vec<u8> = vec![0; m.current_size().0 * 65536];
                m.get_into(0, &mut buffer[..])
                    .expect("can access wasmi memory");
                Some(buffer)
            }
            _ => unreachable!(),
        }
    }
}

impl From<&DiffValue> for wasmi::RuntimeValue {
    fn from(v: &DiffValue) -> Self {
        use wasmi::RuntimeValue::*;
        match *v {
            DiffValue::I32(n) => I32(n),
            DiffValue::I64(n) => I64(n),
            DiffValue::F32(n) => F32(wasmi::nan_preserving_float::F32::from_bits(n)),
            DiffValue::F64(n) => F64(wasmi::nan_preserving_float::F64::from_bits(n)),
            DiffValue::V128(_) | DiffValue::FuncRef { .. } | DiffValue::ExternRef { .. } => {
                unimplemented!()
            }
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

#[test]
fn smoke() {
    crate::oracles::engine::smoke_test_engine(|config| WasmiEngine::new(&config.module_config))
}
