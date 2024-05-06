pub struct Example {
    interface0: exports::same::name::this_name_is_duplicated::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl Example {
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate(&mut store, component)?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Instantiates a pre-instantiated module using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate_pre<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance_pre: &wasmtime::component::InstancePre<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = instance_pre.instantiate(&mut store)?;
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
            let interface0 = exports::same::name::this_name_is_duplicated::Guest::new(
                &mut __exports
                    .instance("same:name/this-name-is-duplicated")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `same:name/this-name-is-duplicated` not present"
                        )
                    })?,
            )?;
            Ok(Example { interface0 })
        }
        pub fn same_name_this_name_is_duplicated(
            &self,
        ) -> &exports::same::name::this_name_is_duplicated::Guest {
            &self.interface0
        }
    }
};
pub mod exports {
    pub mod same {
        pub mod name {
            #[allow(clippy::all)]
            pub mod this_name_is_duplicated {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type ThisNameIsDuplicated = wasmtime::component::ResourceAny;
                pub struct GuestThisNameIsDuplicated<'a> {
                    funcs: &'a Guest,
                }
                pub struct Guest {}
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        Ok(Guest {})
                    }
                }
            }
        }
    }
}
