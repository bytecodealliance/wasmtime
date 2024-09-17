/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`TheWorld`] as well.
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: TheWorldIndices,
}
impl<T> Clone for TheWorldPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> TheWorldPre<_T> {
    /// Creates a new copy of `TheWorldPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = TheWorldIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`TheWorld`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<TheWorld>
    where
        _T: Send,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `the-world`.
///
/// This is an implementation detail of [`TheWorldPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`TheWorld`] as well.
#[derive(Clone)]
pub struct TheWorldIndices {
    interface0: exports::the_name::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`TheWorld::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`TheWorldPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`TheWorldPre::instantiate_async`] to
///   create a [`TheWorld`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`TheWorld::new`].
///
/// * You can also access the guts of instantiation through
///   [`TheWorldIndices::new_instance`] followed
///   by [`TheWorldIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct TheWorld {
    interface0: exports::the_name::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorldIndices {
        /// Creates a new copy of `TheWorldIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            let interface0 = exports::the_name::GuestIndices::new(_component)?;
            Ok(TheWorldIndices { interface0 })
        }
        /// Creates a new instance of [`TheWorldIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`TheWorld`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`TheWorld`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            let interface0 = exports::the_name::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            Ok(TheWorldIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`TheWorld`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheWorld { interface0 })
        }
    }
    impl TheWorld {
        /// Convenience wrapper around [`TheWorldPre::new`] and
        /// [`TheWorldPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheWorld>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`TheWorldIndices::new_instance`] and
        /// [`TheWorldIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let indices = TheWorldIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
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
        #[derive(Clone)]
        pub struct GuestIndices {
            y: wasmtime::component::ComponentExportIndex,
        }
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
                    .export_index(None, "the-name")
                    .ok_or_else(|| {
                        anyhow::anyhow!("no exported instance named `the-name`")
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
                    .get_export(&mut store, None, "the-name")
                    .ok_or_else(|| {
                        anyhow::anyhow!("no exported instance named `the-name`")
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
                                "instance export `the-name` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                let y = lookup("y")?;
                Ok(GuestIndices { y })
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let mut store = store.as_context_mut();
                let _ = &mut store;
                let _instance = instance;
                let y = *_instance.get_typed_func::<(), ()>(&mut store, &self.y)?.func();
                Ok(Guest { y })
            }
        }
        impl Guest {
            pub async fn call_y<S: wasmtime::AsContextMut>(
                &self,
                mut store: S,
            ) -> wasmtime::Result<()>
            where
                <S as wasmtime::AsContext>::Data: Send,
            {
                use tracing::Instrument;
                let span = tracing::span!(
                    tracing::Level::TRACE, "wit-bindgen export", module = "the-name",
                    function = "y",
                );
                let callee = unsafe {
                    wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.y)
                };
                let () = callee
                    .call_async(store.as_context_mut(), ())
                    .instrument(span.clone())
                    .await?;
                callee.post_return_async(store.as_context_mut()).instrument(span).await?;
                Ok(())
            }
        }
    }
}
