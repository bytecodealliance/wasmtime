//! Evaluate an exported Wasm function using the wasmi interpreter.

use crate::generators::{Config, DiffValue, DiffValueType};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{Context, Error, Result};
use wasmtime::{Trap, TrapCode};

/// A wrapper for `wasmi` as a [`DiffEngine`].
pub struct WasmiEngine {
    engine: wasmi::Engine,
}

impl WasmiEngine {
    pub(crate) fn new(config: &mut Config) -> Self {
        let config = &mut config.module_config.config;
        config.reference_types_enabled = false;
        config.simd_enabled = false;
        config.memory64_enabled = false;
        config.bulk_memory_enabled = false;
        config.threads_enabled = false;
        config.max_memories = config.max_memories.min(1);
        config.min_memories = config.min_memories.min(1);
        config.max_tables = config.max_tables.min(1);
        config.min_tables = config.min_tables.min(1);

        Self {
            engine: wasmi::Engine::default(),
        }
    }
}

impl DiffEngine for WasmiEngine {
    fn name(&self) -> &'static str {
        "wasmi"
    }

    fn instantiate(&mut self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let module =
            wasmi::Module::new(&self.engine, wasm).context("unable to validate Wasm module")?;
        let mut store = wasmi::Store::new(&self.engine, ());
        let instance = wasmi::Linker::<()>::new()
            .instantiate(&mut store, &module)
            .and_then(|i| i.start(&mut store))
            .context("unable to instantiate module in wasmi")?;
        Ok(Box::new(WasmiInstance { store, instance }))
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
                .downcast_ref::<wasmi::core::Trap>()
                .expect(&format!("not a trap: {:?}", err)),
        };
        match wasmi.as_code() {
            Some(wasmi::core::TrapCode::StackOverflow) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::StackOverflow))
            }
            Some(wasmi::core::TrapCode::MemoryAccessOutOfBounds) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::MemoryOutOfBounds))
            }
            Some(wasmi::core::TrapCode::Unreachable) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::UnreachableCodeReached))
            }
            Some(wasmi::core::TrapCode::TableAccessOutOfBounds) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::TableOutOfBounds))
            }
            Some(wasmi::core::TrapCode::ElemUninitialized) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::IndirectCallToNull))
            }
            Some(wasmi::core::TrapCode::DivisionByZero) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::IntegerDivisionByZero))
            }
            Some(wasmi::core::TrapCode::IntegerOverflow) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::IntegerOverflow))
            }
            Some(wasmi::core::TrapCode::InvalidConversionToInt) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::BadConversionToInteger))
            }
            Some(wasmi::core::TrapCode::UnexpectedSignature) => {
                assert_eq!(trap.trap_code(), Some(TrapCode::BadSignature))
            }
            None => unreachable!(),
        }
    }

    fn is_stack_overflow(&self, err: &Error) -> bool {
        let trap = match err.downcast_ref::<wasmi::Error>() {
            Some(wasmi::Error::Trap(trap)) => trap,
            Some(_) => return false,
            None => match err.downcast_ref::<wasmi::core::Trap>() {
                Some(trap) => trap,
                None => return false,
            },
        };
        match trap.as_code() {
            Some(wasmi::core::TrapCode::StackOverflow) => true,
            _ => false,
        }
    }
}

/// A wrapper for `wasmi` Wasm instances.
struct WasmiInstance {
    store: wasmi::Store<()>,
    instance: wasmi::Instance,
}

impl DiffInstance for WasmiInstance {
    fn name(&self) -> &'static str {
        "wasmi"
    }

    fn evaluate(
        &mut self,
        function_name: &str,
        arguments: &[DiffValue],
        result_tys: &[DiffValueType],
    ) -> Result<Option<Vec<DiffValue>>> {
        let function = match self
            .instance
            .get_export(&self.store, function_name)
            .unwrap()
        {
            wasmi::Extern::Func(f) => f,
            _ => unreachable!(),
        };
        let arguments: Vec<_> = arguments.iter().map(|x| x.into()).collect();
        let mut results = vec![wasmi::core::Value::I32(0); result_tys.len()];
        function
            .call(&mut self.store, &arguments, &mut results)
            .context("wasmi function trap")?;
        Ok(Some(results.into_iter().map(|x| x.into()).collect()))
    }

    fn get_global(&mut self, name: &str, _ty: DiffValueType) -> Option<DiffValue> {
        match self.instance.get_export(&self.store, name).unwrap() {
            wasmi::Extern::Global(g) => Some(g.get(&self.store).into()),
            _ => unreachable!(),
        }
    }

    fn get_memory(&mut self, name: &str, shared: bool) -> Option<Vec<u8>> {
        assert!(!shared);
        match self.instance.get_export(&self.store, name).unwrap() {
            wasmi::Extern::Memory(m) => Some(m.data(&self.store).to_vec()),
            _ => unreachable!(),
        }
    }
}

impl From<&DiffValue> for wasmi::core::Value {
    fn from(v: &DiffValue) -> Self {
        use wasmi::core::Value::*;
        match *v {
            DiffValue::I32(n) => I32(n),
            DiffValue::I64(n) => I64(n),
            DiffValue::F32(n) => F32(wasmi::core::F32::from_bits(n)),
            DiffValue::F64(n) => F64(wasmi::core::F64::from_bits(n)),
            DiffValue::V128(_) | DiffValue::FuncRef { .. } | DiffValue::ExternRef { .. } => {
                unimplemented!()
            }
        }
    }
}

impl Into<DiffValue> for wasmi::core::Value {
    fn into(self) -> DiffValue {
        use wasmi::core::Value::*;
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
