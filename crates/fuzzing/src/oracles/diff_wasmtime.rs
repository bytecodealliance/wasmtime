//! Evaluate an exported Wasm function using Wasmtime.

use crate::generators::{self, DiffValue};
use crate::oracles::engine::DiffInstance;
use crate::oracles::{compile_module, engine::DiffEngine, instantiate_with_dummy, StoreLimits};
use anyhow::{Context, Result};
use std::hash::Hash;
use std::slice;
use wasmtime::{AsContextMut, Extern, FuncType, Instance, Module, Store, Val};

/// A wrapper for using Wasmtime as a [`DiffEngine`].
pub struct WasmtimeEngine {
    pub(crate) config: generators::Config,
}

impl WasmtimeEngine {
    /// Merely store the configuration; the engine is actually constructed
    /// later. Ideally the store and engine could be built here but
    /// `compile_module` takes a [`generators::Config`]; TODO re-factor this if
    /// that ever changes.
    pub fn new(config: &generators::Config) -> Result<Box<Self>> {
        Ok(Box::new(Self {
            config: config.clone(),
        }))
    }
}

impl DiffEngine for WasmtimeEngine {
    fn name(&self) -> &'static str {
        "wasmtime"
    }

    fn instantiate(&self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let store = self.config.to_store();
        let module = compile_module(store.engine(), wasm, true, &self.config).unwrap();
        let instance = WasmtimeInstance::new(store, module)?;
        Ok(Box::new(instance))
    }
}

/// A wrapper around a Wasmtime instance.
///
/// The Wasmtime engine constructs a new store and compiles an instance of a
/// Wasm module.
pub struct WasmtimeInstance {
    store: Store<StoreLimits>,
    instance: Instance,
}

impl WasmtimeInstance {
    /// Instantiate a new Wasmtime instance.
    pub fn new(mut store: Store<StoreLimits>, module: Module) -> Result<Self> {
        let instance = instantiate_with_dummy(&mut store, &module)
            .context("unable to instantiate module in wasmtime")?;
        Ok(Self { store, instance })
    }

    /// Retrieve the names and types of all exported functions in the instance.
    ///
    /// This is useful for evaluating each exported function with different
    /// values. The [`DiffInstance`] trait asks for the function name and we
    /// need to know the function signature in order to pass in the right
    /// arguments.
    pub fn exported_functions(&mut self) -> Vec<(String, FuncType)> {
        let exported_functions = self
            .instance
            .exports(&mut self.store)
            .map(|e| (e.name().to_owned(), e.into_func()))
            .filter_map(|(n, f)| f.map(|f| (n, f)))
            .collect::<Vec<_>>();
        exported_functions
            .into_iter()
            .map(|(n, f)| (n, f.ty(&self.store)))
            .collect()
    }
}

impl DiffInstance for WasmtimeInstance {
    fn name(&self) -> &'static str {
        "wasmtime"
    }

    fn evaluate(&mut self, function_name: &str, arguments: &[DiffValue]) -> Result<Vec<DiffValue>> {
        let arguments: Vec<_> = arguments.iter().map(Val::from).collect();

        let function = self
            .instance
            .get_func(&mut self.store, function_name)
            .expect("unable to access exported function");
        let ty = function.ty(&self.store);
        let mut results = vec![Val::I32(0); ty.results().len()];
        function.call(&mut self.store, &arguments, &mut results)?;

        let results = results.into_iter().map(Val::into).collect();
        Ok(results)
    }

    fn is_hashable(&self) -> bool {
        true
    }

    fn hash(&mut self, state: &mut std::collections::hash_map::DefaultHasher) -> Result<()> {
        let exports: Vec<_> = self
            .instance
            .exports(self.store.as_context_mut())
            .map(|e| e.into_extern())
            .collect();
        for e in exports {
            match e {
                Extern::Global(g) => {
                    let val: DiffValue = g.get(&mut self.store).into();
                    val.hash(state)
                }
                Extern::Memory(m) => {
                    let data = m.data(&mut self.store);
                    data.hash(state)
                }
                Extern::SharedMemory(m) => {
                    let data = unsafe { slice::from_raw_parts(m.data() as *mut u8, m.data_size()) };
                    data.hash(state)
                }
                Extern::Table(_) => {
                    // TODO: it's unclear whether it is worth it to iterate
                    // through the table and hash the values.
                }
                Extern::Func(_) => {
                    // Note: no need to hash exported functions.
                }
            }
        }
        Ok(())
    }
}

impl From<&DiffValue> for Val {
    fn from(v: &DiffValue) -> Self {
        match *v {
            DiffValue::I32(n) => Val::I32(n),
            DiffValue::I64(n) => Val::I64(n),
            DiffValue::F32(n) => Val::F32(n),
            DiffValue::F64(n) => Val::F64(n),
            DiffValue::V128(n) => Val::V128(n),
        }
    }
}

impl Into<DiffValue> for Val {
    fn into(self) -> DiffValue {
        match self {
            Val::I32(n) => DiffValue::I32(n),
            Val::I64(n) => DiffValue::I64(n),
            Val::F32(n) => DiffValue::F32(n),
            Val::F64(n) => DiffValue::F64(n),
            Val::V128(n) => DiffValue::V128(n),
            Val::FuncRef(_) => unimplemented!(),
            Val::ExternRef(_) => unimplemented!(),
        }
    }
}
