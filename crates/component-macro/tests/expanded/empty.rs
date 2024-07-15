/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `empty`.
///
/// This structure is created through [`EmptyPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct EmptyPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
}
impl<T> Clone for EmptyPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `empty`.
///
/// This structure is created through either
/// [`Empty::instantiate`] or by first creating
/// a [`EmptyPre`] followed by using
/// [`EmptyPre::instantiate`].
pub struct Empty {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> EmptyPre<_T> {
        /// Creates a new copy of `EmptyPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            Ok(EmptyPre { instance_pre })
        }
        /// Instantiates a new instance of [`Empty`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub fn instantiate(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Empty> {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate(&mut store)?;
            Ok(Empty {})
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl Empty {
        /// Convenience wrapper around [`EmptyPre::new`] and
        /// [`EmptyPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Empty> {
            let pre = linker.instantiate_pre(component)?;
            EmptyPre::new(pre)?.instantiate(store)
        }
    }
};
