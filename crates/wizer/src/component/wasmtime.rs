use crate::Wizer;
use crate::component::ComponentInstanceState;
use wasmtime::component::{
    Component, ComponentExportIndex, Instance, Lift, WasmList, types::ComponentItem,
};
use wasmtime::{Result, Store, error::Context as _, format_err};

#[cfg(feature = "wasmprinter")]
use wasmtime::ToWasmtimeResult as _;

impl Wizer {
    /// Same as [`Wizer::run`], except for components.
    pub async fn run_component<T: Send>(
        &self,
        store: &mut Store<T>,
        wasm: &[u8],
        instantiate: impl AsyncFnOnce(&mut Store<T>, &Component) -> Result<Instance>,
    ) -> wasmtime::Result<Vec<u8>> {
        let (cx, instrumented_wasm) = self.instrument_component(wasm)?;

        #[cfg(feature = "wasmprinter")]
        log::debug!(
            "instrumented wasm: {}",
            wasmprinter::print_bytes(&instrumented_wasm).to_wasmtime_result()?,
        );

        let engine = store.engine();
        let component = Component::new(engine, &instrumented_wasm)
            .context("failed to compile the Wasm component")?;
        let index = self.validate_component_init_func(&component)?;

        let instance = instantiate(store, &component).await?;
        self.initialize_component(store, &instance, index).await?;
        self.snapshot_component(cx, &mut WasmtimeWizerComponent { store, instance })
            .await
    }

    fn validate_component_init_func(
        &self,
        component: &Component,
    ) -> wasmtime::Result<ComponentExportIndex> {
        let init_func = self.get_init_func();
        let (ty, index) = component
            .get_export(None, init_func)
            .ok_or_else(|| format_err!("the component does export the function `{init_func}`"))?;

        let ty = match ty {
            ComponentItem::ComponentFunc(ty) => ty,
            _ => wasmtime::bail!("the component's `{init_func}` export is not a function",),
        };

        if ty.params().len() != 0 || ty.results().len() != 0 {
            wasmtime::bail!(
                "the component's `{init_func}` function export does not have type `[] -> []`",
            );
        }
        Ok(index)
    }

    async fn initialize_component<T: Send>(
        &self,
        store: &mut Store<T>,
        instance: &Instance,
        index: ComponentExportIndex,
    ) -> wasmtime::Result<()> {
        let init_func = instance
            .get_typed_func::<(), ()>(&mut *store, index)
            .expect("checked by `validate_init_func`");
        init_func
            .call_async(&mut *store, ())
            .await
            .with_context(|| format!("the initialization function trapped"))?;
        init_func
            .post_return_async(&mut *store)
            .await
            .context("failed to call post-return")?;

        Ok(())
    }
}

/// Impementation of [`InstanceState`] backed by Wasmtime.
pub struct WasmtimeWizerComponent<'a, T: 'static> {
    /// The Wasmtime-based store that owns the `instance` field.
    pub store: &'a mut Store<T>,
    /// The instance that this will load state from.
    pub instance: Instance,
}

impl<T: Send> WasmtimeWizerComponent<'_, T> {
    async fn call_func<R, R2>(
        &mut self,
        instance: &str,
        func: &str,
        use_ret: impl FnOnce(&mut Store<T>, R) -> R2,
    ) -> R2
    where
        R: Lift + 'static,
    {
        log::debug!("invoking {instance}#{func}");
        let (_, instance_export) = self
            .instance
            .get_export(&mut *self.store, None, instance)
            .unwrap();
        let (_, func_export) = self
            .instance
            .get_export(&mut *self.store, Some(&instance_export), func)
            .unwrap();
        let func = self
            .instance
            .get_typed_func::<(), (R,)>(&mut *self.store, func_export)
            .unwrap();
        let ret = func.call_async(&mut *self.store, ()).await.unwrap().0;
        let ret = use_ret(&mut *self.store, ret);
        func.post_return_async(&mut *self.store).await.unwrap();
        ret
    }
}

impl<T: Send> ComponentInstanceState for WasmtimeWizerComponent<'_, T> {
    async fn call_func_ret_list_u8(
        &mut self,
        instance: &str,
        func: &str,
        contents: impl FnOnce(&[u8]) + Send,
    ) {
        self.call_func(instance, func, |store, list: WasmList<u8>| {
            contents(list.as_le_slice(&store));
        })
        .await
    }

    async fn call_func_ret_s32(&mut self, instance: &str, func: &str) -> i32 {
        self.call_func(instance, func, |_, r| r).await
    }

    async fn call_func_ret_s64(&mut self, instance: &str, func: &str) -> i64 {
        self.call_func(instance, func, |_, r| r).await
    }

    async fn call_func_ret_f32(&mut self, instance: &str, func: &str) -> u32 {
        self.call_func(instance, func, |_, r: f32| r.to_bits())
            .await
    }

    async fn call_func_ret_f64(&mut self, instance: &str, func: &str) -> u64 {
        self.call_func(instance, func, |_, r: f64| r.to_bits())
            .await
    }
}
