/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `foo`.
///
/// This structure is created through [`FooPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct FooPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    new: wasmtime::component::ComponentExportIndex,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `foo`.
///
/// This structure is created through either
/// [`Foo::instantiate_async`] or by first creating
/// a [`FooPre`] followed by using
/// [`FooPre::instantiate_async`].
pub struct Foo {
    new: wasmtime::component::Func,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> FooPre<_T> {
        /// Creates a new copy of `FooPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let new = _component
                .export_index(None, "new")
                .ok_or_else(|| anyhow::anyhow!("no function export `new` found"))?
                .1;
            Ok(FooPre { instance_pre, new })
        }
        /// Instantiates a new instance of [`Foo`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Foo>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let new = *_instance.get_typed_func::<(), ()>(&mut store, &self.new)?.func();
            Ok(Foo { new })
        }
    }
    impl Foo {
        /// Convenience wrapper around [`FooPre::new`] and
        /// [`FooPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Foo>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            FooPre::new(pre)?.instantiate_async(store).await
        }
        pub async fn call_new<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
        ) -> wasmtime::Result<()>
        where
            <S as wasmtime::AsContext>::Data: Send,
        {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.new)
            };
            let () = callee.call_async(store.as_context_mut(), ()).await?;
            callee.post_return_async(store.as_context_mut()).await?;
            Ok(())
        }
    }
};
