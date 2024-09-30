//! Evaluate an exported Wasm function using the wasmi interpreter.

use crate::generators::{Config, DiffValue, DiffValueType};
use crate::oracles::engine::{DiffEngine, DiffInstance};
use anyhow::{Context, Error, Result};
use wasmtime::Trap;

/// A wrapper for `wasmi` as a [`DiffEngine`].
pub struct WasmiEngine {
    engine: wasmi::Engine,
}

impl WasmiEngine {
    pub(crate) fn new(config: &mut Config) -> Self {
        let config = &mut config.module_config.config;
        // Force generated Wasm modules to never have features that Wasmi doesn't support.
        config.simd_enabled = false;
        config.relaxed_simd_enabled = false;
        config.memory64_enabled = false;
        config.threads_enabled = false;
        config.exceptions_enabled = false;
        config.max_memories = config.max_memories.min(1);
        config.min_memories = config.min_memories.min(1);

        let mut wasmi_config = wasmi::Config::default();
        wasmi_config
            .consume_fuel(false)
            .floats(true)
            .wasm_mutable_global(true)
            .wasm_sign_extension(config.sign_extension_ops_enabled)
            .wasm_saturating_float_to_int(config.saturating_float_to_int_enabled)
            .wasm_multi_value(config.multi_value_enabled)
            .wasm_bulk_memory(config.bulk_memory_enabled)
            .wasm_reference_types(config.reference_types_enabled)
            .wasm_tail_call(config.tail_call_enabled)
            .wasm_extended_const(true);
        Self {
            engine: wasmi::Engine::new(&wasmi_config),
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
        let instance = wasmi::Linker::<()>::new(&self.engine)
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
                    *trap,
                    Trap::MemoryOutOfBounds,
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

            Some(other) => panic!("unexpected wasmi error: {other}"),

            None => err
                .downcast_ref::<wasmi::core::Trap>()
                .expect(&format!("not a trap: {err:?}")),
        };
        assert!(wasmi.trap_code().is_some());
        assert_eq!(
            wasmi_to_wasmtime_trap_code(wasmi.trap_code().unwrap()),
            *trap
        );
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
        matches!(trap.trap_code(), Some(wasmi::core::TrapCode::StackOverflow))
    }
}

/// Converts `wasmi` trap code to `wasmtime` trap code.
fn wasmi_to_wasmtime_trap_code(trap: wasmi::core::TrapCode) -> Trap {
    use wasmi::core::TrapCode;
    match trap {
        TrapCode::UnreachableCodeReached => Trap::UnreachableCodeReached,
        TrapCode::MemoryOutOfBounds => Trap::MemoryOutOfBounds,
        TrapCode::TableOutOfBounds => Trap::TableOutOfBounds,
        TrapCode::IndirectCallToNull => Trap::IndirectCallToNull,
        TrapCode::IntegerDivisionByZero => Trap::IntegerDivisionByZero,
        TrapCode::IntegerOverflow => Trap::IntegerOverflow,
        TrapCode::BadConversionToInteger => Trap::BadConversionToInteger,
        TrapCode::StackOverflow => Trap::StackOverflow,
        TrapCode::BadSignature => Trap::BadSignature,
        TrapCode::OutOfFuel => unimplemented!("built-in fuel metering is unused"),
        TrapCode::GrowthOperationLimited => unimplemented!("resource limiter is unused"),
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
        let function = self
            .instance
            .get_export(&self.store, function_name)
            .and_then(wasmi::Extern::into_func)
            .unwrap();
        let arguments: Vec<_> = arguments.iter().map(|x| x.into()).collect();
        let mut results = vec![wasmi::Value::I32(0); result_tys.len()];
        function
            .call(&mut self.store, &arguments, &mut results)
            .context("wasmi function trap")?;
        Ok(Some(results.into_iter().map(Into::into).collect()))
    }

    fn get_global(&mut self, name: &str, _ty: DiffValueType) -> Option<DiffValue> {
        Some(
            self.instance
                .get_export(&self.store, name)
                .unwrap()
                .into_global()
                .unwrap()
                .get(&self.store)
                .into(),
        )
    }

    fn get_memory(&mut self, name: &str, shared: bool) -> Option<Vec<u8>> {
        assert!(!shared);
        Some(
            self.instance
                .get_export(&self.store, name)
                .unwrap()
                .into_memory()
                .unwrap()
                .data(&self.store)
                .to_vec(),
        )
    }
}

impl From<&DiffValue> for wasmi::Value {
    fn from(v: &DiffValue) -> Self {
        use wasmi::Value as WasmiValue;
        match *v {
            DiffValue::I32(n) => WasmiValue::I32(n),
            DiffValue::I64(n) => WasmiValue::I64(n),
            DiffValue::F32(n) => WasmiValue::F32(wasmi::core::F32::from_bits(n)),
            DiffValue::F64(n) => WasmiValue::F64(wasmi::core::F64::from_bits(n)),
            DiffValue::V128(_) => unimplemented!(),
            DiffValue::FuncRef { null } => {
                assert!(null);
                WasmiValue::FuncRef(wasmi::FuncRef::null())
            }
            DiffValue::ExternRef { null } => {
                assert!(null);
                WasmiValue::ExternRef(wasmi::ExternRef::null())
            }
            DiffValue::AnyRef { .. } => unimplemented!(),
        }
    }
}

impl From<wasmi::Value> for DiffValue {
    fn from(value: wasmi::Value) -> Self {
        use wasmi::Value as WasmiValue;
        match value {
            WasmiValue::I32(n) => DiffValue::I32(n),
            WasmiValue::I64(n) => DiffValue::I64(n),
            WasmiValue::F32(n) => DiffValue::F32(n.to_bits()),
            WasmiValue::F64(n) => DiffValue::F64(n.to_bits()),
            WasmiValue::FuncRef(f) => DiffValue::FuncRef { null: f.is_null() },
            WasmiValue::ExternRef(e) => DiffValue::ExternRef { null: e.is_null() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::oracles::engine::smoke_test_engine(|_, config| Ok(WasmiEngine::new(config)))
    }
}
