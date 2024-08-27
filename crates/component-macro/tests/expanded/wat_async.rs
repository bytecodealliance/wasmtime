/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `example`.
///
/// This structure is created through [`ExamplePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Example`] as well.
pub struct ExamplePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: ExampleIndices,
}
impl<T> Clone for ExamplePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> ExamplePre<_T> {
    /// Creates a new copy of `ExamplePre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = ExampleIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
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
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `example`.
///
/// This is an implementation detail of [`ExamplePre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Example`] as well.
#[derive(Clone)]
pub struct ExampleIndices {
    interface0: exports::same::name::this_name_is_duplicated::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `example`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Example::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`ExamplePre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`ExamplePre::instantiate_async`] to
///   create a [`Example`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Example::new_instance`]
///
/// * You can also access the guts of instantiation through
///   [`ExampleIndices::new_instance`] followed
///   by [`ExampleIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Example {
    interface0: exports::same::name::this_name_is_duplicated::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl ExampleIndices {
        /// Creates a new copy of `ExampleIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            let interface0 = exports::same::name::this_name_is_duplicated::GuestIndices::new(
                _component,
            )?;
            Ok(ExampleIndices { interface0 })
        }
        /// Creates a new instance of [`ExampleIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`Example`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`Example`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            let interface0 = exports::same::name::this_name_is_duplicated::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            Ok(ExampleIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Example`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Example> {
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(Example { interface0 })
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
        /// Convenience wrapper around [`ExampleIndices::new_instance`] and
        /// [`ExampleIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Example> {
            let indices = ExampleIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
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
                pub struct GuestIndices {}
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestIndices> {
                        let (_, instance) = component
                            .export_index(None, "same:name/this-name-is-duplicated")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `same:name/this-name-is-duplicated`"
                                )
                            })?;
                        Self::_new(|name| {
                            component.export_index(Some(&instance), name).map(|p| p.1)
                        })
                    }
                    /// This constructor is similar to [`GuestIndices::new`] except that it
                    /// performs string lookups after instantiation time.
                    pub fn new_instance(
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance_export = instance
                            .get_export(
                                &mut store,
                                None,
                                "same:name/this-name-is-duplicated",
                            )
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `same:name/this-name-is-duplicated`"
                                )
                            })?;
                        Self::_new(|name| {
                            instance.get_export(&mut store, Some(&instance_export), name)
                        })
                    }
                    fn _new(
                        mut lookup: impl FnMut(
                            &str,
                        ) -> Option<wasmtime::component::ComponentExportIndex>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let mut lookup = move |name| {
                            lookup(name)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `same:name/this-name-is-duplicated` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        Ok(GuestIndices {})
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
