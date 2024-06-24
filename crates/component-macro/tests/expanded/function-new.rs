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
impl<T> Clone for FooPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            new: self.new.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `foo`.
///
/// This structure is created through either
/// [`Foo::instantiate`] or by first creating
/// a [`FooPre`] followed by using
/// [`FooPre::instantiate`].
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
        pub fn instantiate(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Foo> {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate(&mut store)?;
            let new = *_instance.get_typed_func::<(), ()>(&mut store, &self.new)?.func();
            Ok(Foo { new })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl Foo {
        /// Convenience wrapper around [`FooPre::new`] and
        /// [`FooPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Foo> {
            let pre = linker.instantiate_pre(component)?;
            FooPre::new(pre)?.instantiate(store)
        }
        pub fn call_new<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
        ) -> wasmtime::Result<()> {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.new)
            };
            let () = callee.call(store.as_context_mut(), ())?;
            callee.post_return(store.as_context_mut())?;
            Ok(())
        }
    }
};
