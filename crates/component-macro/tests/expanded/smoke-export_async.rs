pub struct TheWorld {
    interface0: exports::the_name::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorld {
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub async fn instantiate_async<T: Send>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate_async(&mut store, component).await?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Instantiates a pre-instantiated module using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub async fn instantiate_pre<T: Send>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance_pre: &wasmtime::component::InstancePre<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = instance_pre.instantiate_async(&mut store).await?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Low-level creation wrapper for wrapping up the exports
        /// of the `instance` provided in this structure of wasm
        /// exports.
        ///
        /// This function will extract exports from the `instance`
        /// defined within `store` and wrap them all up in the
        /// returned structure which can be used to interact with
        /// the wasm module.
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let mut store = store.as_context_mut();
            let mut exports = instance.exports(&mut store);
            let mut __exports = exports.root();
            let interface0 = exports::the_name::Guest::new(
                &mut __exports
                    .instance("the-name")
                    .ok_or_else(|| {
                        anyhow::anyhow!("exported instance `the-name` not present")
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn the_name(&self) -> &exports::the_name::Guest {
            &self.interface0
        }
    }
};
pub mod exports {
    #[allow(clippy::all)]
    pub mod the_name {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::anyhow;
        pub struct Guest {
            y: wasmtime::component::Func,
        }
        impl Guest {
            pub fn new(
                __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
            ) -> wasmtime::Result<Guest> {
                let y = *__exports.typed_func::<(), ()>("y")?.func();
                Ok(Guest { y })
            }
            pub async fn call_y<S: wasmtime::AsContextMut>(
                &self,
                mut store: S,
            ) -> wasmtime::Result<()>
            where
                <S as wasmtime::AsContext>::Data: Send,
            {
                let callee = unsafe {
                    wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.y)
                };
                let () = callee.call_async(store.as_context_mut(), ()).await?;
                callee.post_return_async(store.as_context_mut()).await?;
                Ok(())
            }
        }
    }
}
