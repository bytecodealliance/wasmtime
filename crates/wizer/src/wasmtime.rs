use crate::{InstanceState, SnapshotVal, Wizer};
use wasmtime::error::Context;
use wasmtime::{Extern, Instance, Module, Result, Store, Val};

impl Wizer {
    /// Initialize the given Wasm, snapshot it, and return the serialized
    /// snapshot as a new, pre-initialized Wasm module.
    pub async fn run<T: Send>(
        &self,
        store: &mut Store<T>,
        wasm: &[u8],
        instantiate: impl AsyncFnOnce(&mut Store<T>, &Module) -> Result<wasmtime::Instance>,
    ) -> wasmtime::Result<Vec<u8>> {
        let (cx, instrumented_wasm) = self.instrument(wasm)?;

        let engine = store.engine();
        let module = wasmtime::Module::new(engine, &instrumented_wasm)
            .context("failed to compile the Wasm module")?;
        self.validate_init_func(&module)?;

        let instance = instantiate(store, &module).await?;
        self.initialize(store, &instance).await?;
        self.snapshot(cx, &mut WasmtimeWizer { store, instance })
            .await
    }

    /// Check that the module exports an initialization function, and that the
    /// function has the correct type.
    fn validate_init_func(&self, module: &wasmtime::Module) -> wasmtime::Result<()> {
        log::debug!("Validating the exported initialization function");
        match module.get_export(self.get_init_func()) {
            Some(wasmtime::ExternType::Func(func_ty)) => {
                if func_ty.params().len() != 0 || func_ty.results().len() != 0 {
                    wasmtime::bail!(
                        "the Wasm module's `{}` function export does not have type `[] -> []`",
                        self.get_init_func()
                    );
                }
            }
            Some(_) => wasmtime::bail!(
                "the Wasm module's `{}` export is not a function",
                self.get_init_func()
            ),
            None => wasmtime::bail!(
                "the Wasm module does not have a `{}` export",
                self.get_init_func()
            ),
        }
        Ok(())
    }

    /// Instantiate the module and call its initialization function.
    async fn initialize<T: Send>(
        &self,
        store: &mut Store<T>,
        instance: &wasmtime::Instance,
    ) -> wasmtime::Result<()> {
        log::debug!("Calling the initialization function");

        if let Some(export) = instance.get_export(&mut *store, "_initialize") {
            if let Extern::Func(func) = export {
                func.typed::<(), ()>(&store)?
                    .call_async(&mut *store, ())
                    .await
                    .context("calling the Reactor initialization function")?;

                if self.get_init_func() == "_initialize" {
                    // Don't run `_initialize` twice if the it was explicitly
                    // requested as the init function.
                    return Ok(());
                }
            }
        }

        let init_func = instance
            .get_typed_func::<(), ()>(&mut *store, self.get_init_func())
            .expect("checked by `validate_init_func`");
        init_func
            .call_async(&mut *store, ())
            .await
            .with_context(|| format!("the `{}` function trapped", self.get_init_func()))?;

        Ok(())
    }
}

/// Impementation of [`InstanceState`] backed by Wasmtime.
pub struct WasmtimeWizer<'a, T: 'static> {
    /// The Wasmtime-based store that owns the `instance` field.
    pub store: &'a mut Store<T>,
    /// The instance that this will load state from.
    pub instance: Instance,
}

impl<T: Send> InstanceState for WasmtimeWizer<'_, T> {
    async fn global_get(&mut self, name: &str) -> SnapshotVal {
        let global = self.instance.get_global(&mut *self.store, name).unwrap();
        match global.get(&mut *self.store) {
            Val::I32(x) => SnapshotVal::I32(x),
            Val::I64(x) => SnapshotVal::I64(x),
            Val::F32(x) => SnapshotVal::F32(x),
            Val::F64(x) => SnapshotVal::F64(x),
            Val::V128(x) => SnapshotVal::V128(x.as_u128()),
            _ => panic!("unsupported global value type"),
        }
    }

    async fn memory_contents(&mut self, name: &str, contents: impl FnOnce(&[u8]) + Send) {
        let memory = self.instance.get_memory(&mut *self.store, name).unwrap();
        contents(memory.data(&self.store))
    }
}
