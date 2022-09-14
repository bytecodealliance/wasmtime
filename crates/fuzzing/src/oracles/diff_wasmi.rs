//! Evaluate an exported Wasm function using the wasmi interpreter.

use crate::generators::{Config, DiffValue, DiffValueType};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{Context, Error, Result};
use wasmtime::{Trap, TrapCode};

/// A wrapper for `wasmi` as a [`DiffEngine`].
pub struct WasmiEngine;

impl WasmiEngine {
    pub(crate) fn new(config: &mut Config) -> Self {
        let config = &mut config.module_config.config;
        config.reference_types_enabled = false;
        config.simd_enabled = false;
        config.multi_value_enabled = false;
        config.saturating_float_to_int_enabled = false;
        config.sign_extension_enabled = false;
        config.memory64_enabled = false;
        config.bulk_memory_enabled = false;
        config.threads_enabled = false;
        config.max_memories = config.max_memories.min(1);
        config.min_memories = config.min_memories.min(1);
        config.max_tables = config.max_tables.min(1);
        config.min_tables = config.min_tables.min(1);

        Self
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
        let instance = instance.run_start(&mut wasmi::NopExternals)?;
        Ok(Box::new(WasmiInstance { module, instance }))
    }

    fn assert_error_match(&self, trap: &Trap, err: &Error) {
        // Acquire a `wasmi::Trap` from the wasmi error which we'll use to
        // assert that it has the same kind of trap as the wasmtime-based trap.
        let wasmi = match err.downcast_ref::<wasmi::Error>() {
            Some(wasmi::Error::Trap(trap)) => trap,

            // Out-of-bounds data segments turn into this category which
            // Wasmtime reports as a `MemoryOutOfBounds`.
            Some(wasmi::Error::Memory(msg)) => {
                assert_eq!(
                    trap.trap_code(),
                    Some(TrapCode::MemoryOutOfBounds),
                    "wasmtime error did not match wasmi: {msg}"
                );
                return;
            }

            // Ignore this for now, looks like "elements segment does not fit"
            // falls into this category and to avoid doing string matching this
            // is just ignored.
            Some(wasmi::Error::Instantiation(msg)) => {
                log::debug!("ignoring wasmi instantiation error: {msg}");
                return;
            }

            Some(other) => panic!("unexpected wasmi error: {}", other),

            None => err
                .downcast_ref::<wasmi::Trap>()
                .expect(&format!("not a trap: {:?}", err)),
        };
        match wasmi.kind() {
            wasmi::TrapKind::StackOverflow => {
                assert_eq!(trap.trap_code(), Some(TrapCode::StackOverflow))
            }
            wasmi::TrapKind::MemoryAccessOutOfBounds => {
                assert_eq!(trap.trap_code(), Some(TrapCode::MemoryOutOfBounds))
            }
            wasmi::TrapKind::Unreachable => {
                assert_eq!(trap.trap_code(), Some(TrapCode::UnreachableCodeReached))
            }
            wasmi::TrapKind::TableAccessOutOfBounds => {
                assert_eq!(trap.trap_code(), Some(TrapCode::TableOutOfBounds))
            }
            wasmi::TrapKind::ElemUninitialized => {
                assert_eq!(trap.trap_code(), Some(TrapCode::IndirectCallToNull))
            }
            wasmi::TrapKind::DivisionByZero => {
                assert_eq!(trap.trap_code(), Some(TrapCode::IntegerDivisionByZero))
            }
            wasmi::TrapKind::IntegerOverflow => {
                assert_eq!(trap.trap_code(), Some(TrapCode::IntegerOverflow))
            }
            wasmi::TrapKind::InvalidConversionToInt => {
                assert_eq!(trap.trap_code(), Some(TrapCode::BadConversionToInteger))
            }
            wasmi::TrapKind::UnexpectedSignature => {
                assert_eq!(trap.trap_code(), Some(TrapCode::BadSignature))
            }
            wasmi::TrapKind::Host(_) => unreachable!(),
        }
    }

    fn is_stack_overflow(&self, err: &Error) -> bool {
        let trap = match err.downcast_ref::<wasmi::Error>() {
            Some(wasmi::Error::Trap(trap)) => trap,
            Some(_) => return false,
            None => match err.downcast_ref::<wasmi::Trap>() {
                Some(trap) => trap,
                None => return false,
            },
        };
        match trap.kind() {
            wasmi::TrapKind::StackOverflow => true,
            _ => false,
        }
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
    crate::oracles::engine::smoke_test_engine(|_, config| Ok(WasmiEngine::new(config)))
}
