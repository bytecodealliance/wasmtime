/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `foo`.
///
/// This structure is created through [`FooPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct FooPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
}
impl<T> Clone for FooPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
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
pub struct Foo {}
pub trait FooImports {
    fn foo(&mut self) -> ();
}
pub trait FooImportsGetHost<
    T,
>: Fn(T) -> <Self as FooImportsGetHost<T>>::Host + Send + Sync + Copy + 'static {
    type Host: FooImports;
}
impl<F, T, O> FooImportsGetHost<T> for F
where
    F: Fn(T) -> O + Send + Sync + Copy + 'static,
    O: FooImports,
{
    type Host = O;
}
impl<_T: FooImports + ?Sized> FooImports for &mut _T {
    fn foo(&mut self) -> () {
        FooImports::foo(*self)
    }
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
            Ok(FooPre { instance_pre })
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
            Ok(Foo {})
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
        pub fn add_to_linker_imports_get_host<T>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: impl for<'a> FooImportsGetHost<&'a mut T>,
        ) -> wasmtime::Result<()> {
            let mut linker = linker.root();
            linker
                .func_wrap(
                    "foo",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = FooImports::foo(host);
                        Ok(r)
                    },
                )?;
            Ok(())
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: FooImports,
        {
            Self::add_to_linker_imports_get_host(linker, get)?;
            Ok(())
        }
    }
};
