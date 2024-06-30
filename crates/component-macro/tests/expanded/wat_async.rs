/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `example`.
///
/// This structure is created through [`ExamplePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct ExamplePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::same::name::this_name_is_duplicated::GuestPre,
}
impl<T> Clone for ExamplePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            interface0: self.interface0.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `example`.
///
/// This structure is created through either
/// [`Example::instantiate_async`] or by first creating
/// a [`ExamplePre`] followed by using
/// [`ExamplePre::instantiate_async`].
pub struct Example {
    interface0: exports::same::name::this_name_is_duplicated::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> ExamplePre<_T> {
        /// Creates a new copy of `ExamplePre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::same::name::this_name_is_duplicated::GuestPre::new(
                _component,
            )?;
            Ok(ExamplePre {
                instance_pre,
                interface0,
            })
        }
        /// Instantiates a new instance of [`Example`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Example>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(Example { interface0 })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl Example {
        /// Convenience wrapper around [`ExamplePre::new`] and
        /// [`ExamplePre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Example>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            ExamplePre::new(pre)?.instantiate_async(store).await
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
                #[derive(Clone)]
                pub struct GuestPre {}
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "same:name/this-name-is-duplicated")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `same:name/this-name-is-duplicated`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `same:name/this-name-is-duplicated` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        Ok(GuestPre {})
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        Ok(Guest {})
                    }
                }
                impl Guest {}
            }
        }
    }
}
