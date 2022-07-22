//! Evaluate an exported Wasm function using Wasmtime.

use crate::generators::{self, DiffValue, InstanceAllocationStrategy};
use crate::oracles::engine::DiffInstance;
use crate::oracles::{compile_module, engine::DiffEngine, instantiate_with_dummy, StoreLimits};
use anyhow::{Context, Result};
use arbitrary::Unstructured;
use std::cell::RefCell;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::slice;
use wasmtime::{Extern, Instance, Store, Val};

use super::engine::DiffIgnoreError;

/// A wrapper for using Wasmtime as a [`DiffEngine`].
pub struct WasmtimeEngine {
    config: generators::Config,
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

    /// Construct a new Wasmtime engine with a randomly-generated configuration
    /// that is compatible with `original_config`.
    pub fn new_with_compatible_config(
        input: &mut Unstructured<'_>,
        original_config: &generators::Config,
    ) -> Result<Box<Self>> {
        // Generate a completely new Wasmtime configuration leaving the module
        // configuration the same.
        let mut new_config = generators::Config {
            module_config: original_config.module_config.clone(),
            wasmtime: input.arbitrary()?,
        };

        // Use the same allocation strategy between the two configs.
        //
        // Ideally this wouldn't be necessary, but if the lhs is using ondemand
        // and the rhs is using the pooling allocator (or vice versa), then
        // the module may have been generated in such a way that is incompatible
        // with the other allocation strategy.
        //
        // We can remove this in the future when it's possible to access the
        // fields of `wasm_smith::Module` to constrain the pooling allocator
        // based on what was actually generated.
        new_config.wasmtime.strategy = original_config.wasmtime.strategy.clone();
        if let InstanceAllocationStrategy::Pooling { .. } = &new_config.wasmtime.strategy {
            // Also use the same memory configuration when using the pooling allocator
            new_config.wasmtime.memory_config = original_config.wasmtime.memory_config.clone();
        }

        WasmtimeEngine::new(&new_config)
    }
}

impl DiffEngine for WasmtimeEngine {
    fn instantiate(&self, wasm: &[u8]) -> Result<Box<dyn DiffInstance>> {
        let mut store = self.config.to_store();
        let module = compile_module(store.engine(), wasm, true, &self.config).ok_or(
            DiffIgnoreError("unable to compile module in wasmtime".into()),
        )?;
        let instance = instantiate_with_dummy(&mut store, &module)
            .context("unable to instantiate module in wasmtime")?;
        let instance = WasmtimeInstance {
            store: RefCell::new(store),
            instance,
        };
        Ok(Box::new(instance))
    }
}

/// A wrapper around a Wasmtime instance.
///
/// The Wasmtime engine constructs a new store and compiles an instance of a
/// Wasm module. The store is hidden in a [`RefCell`] so that we can hash, which
/// does not modify the [`Store`] even though the API makes it appear so.
struct WasmtimeInstance {
    store: RefCell<Store<StoreLimits>>,
    instance: Instance,
}

impl DiffInstance for WasmtimeInstance {
    fn name(&self) -> &'static str {
        "wasmtime"
    }

    fn evaluate(&mut self, function_name: &str, arguments: &[DiffValue]) -> Result<Vec<DiffValue>> {
        let arguments: Vec<_> = arguments.iter().map(Val::from).collect();

        let function = self
            .instance
            .get_func(self.store.borrow_mut().deref_mut(), function_name)
            .expect("unable to access exported function");
        let ty = function.ty(self.store.borrow().deref());
        let mut results = vec![Val::I32(0); ty.results().len()];
        function.call(
            self.store.borrow_mut().deref_mut(),
            &arguments,
            &mut results,
        )?;

        let results = results.into_iter().map(Val::into).collect();
        Ok(results)
    }

    fn is_hashable(&self) -> bool {
        true
    }

    fn hash(&self, state: &mut std::collections::hash_map::DefaultHasher) -> Result<()> {
        for e in self.instance.exports(self.store.borrow_mut().deref_mut()) {
            match e.into_extern() {
                Extern::Global(g) => {
                    let val: DiffValue = g.get(self.store.borrow_mut().deref_mut()).into();
                    val.hash(state)
                }
                Extern::Memory(m) => {
                    let mut store = self.store.borrow_mut();
                    let data = m.data(store.deref_mut());
                    data.hash(state)
                }
                Extern::SharedMemory(m) => {
                    let data = unsafe { slice::from_raw_parts(m.data() as *mut u8, m.data_size()) };
                    data.hash(state)
                }
                Extern::Table(_) => {
                    // TODO: it's unclear whether it is worth it to iterate
                    // through the table and hash the values.
                    todo!()
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
