/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `empty`.
///
/// This structure is created through [`EmptyPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Empty`] as well.
pub struct EmptyPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: EmptyIndices,
}
impl<T> Clone for EmptyPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> EmptyPre<_T> {
    /// Creates a new copy of `EmptyPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = EmptyIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`Empty`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Empty>
    where
        _T: Send,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `empty`.
///
/// This is an implementation detail of [`EmptyPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Empty`] as well.
#[derive(Clone)]
pub struct EmptyIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `empty`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Empty::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`EmptyPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`EmptyPre::instantiate_async`] to
///   create a [`Empty`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Empty::new`].
///
/// * You can also access the guts of instantiation through
///   [`EmptyIndices::new_instance`] followed
///   by [`EmptyIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Empty {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl EmptyIndices {
        /// Creates a new copy of `EmptyIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            Ok(EmptyIndices {})
        }
        /// Creates a new instance of [`EmptyIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`Empty`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`Empty`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            Ok(EmptyIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Empty`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Empty> {
            let _instance = instance;
            Ok(Empty {})
        }
    }
    impl Empty {
        /// Convenience wrapper around [`EmptyPre::new`] and
        /// [`EmptyPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Empty>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            EmptyPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`EmptyIndices::new_instance`] and
        /// [`EmptyIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Empty> {
            let indices = EmptyIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
    }
};
