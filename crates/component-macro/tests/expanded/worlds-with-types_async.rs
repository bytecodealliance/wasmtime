pub type U = foo::foo::i::T;
const _: () = {
    assert!(2 == < U as wasmtime::component::ComponentType >::SIZE32);
    assert!(2 == < U as wasmtime::component::ComponentType >::ALIGN32);
};
pub type T = u32;
const _: () = {
    assert!(4 == < T as wasmtime::component::ComponentType >::SIZE32);
    assert!(4 == < T as wasmtime::component::ComponentType >::ALIGN32);
};
#[derive(wasmtime::component::ComponentType)]
#[derive(wasmtime::component::Lift)]
#[derive(wasmtime::component::Lower)]
#[component(record)]
#[derive(Clone, Copy)]
pub struct R {}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("R").finish()
    }
}
const _: () = {
    assert!(0 == < R as wasmtime::component::ComponentType >::SIZE32);
    assert!(1 == < R as wasmtime::component::ComponentType >::ALIGN32);
};
/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `foo`.
///
/// This structure is created through [`FooPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct FooPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    f: wasmtime::component::ComponentExportIndex,
}
impl<T> Clone for FooPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            f: self.f.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `foo`.
///
/// This structure is created through either
/// [`Foo::instantiate_async`] or by first creating
/// a [`FooPre`] followed by using
/// [`FooPre::instantiate_async`].
pub struct Foo {
    f: wasmtime::component::Func,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> FooPre<_T> {
        /// Creates a new copy of `FooPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let f = _component
                .export_index(None, "f")
                .ok_or_else(|| anyhow::anyhow!("no function export `f` found"))?
                .1;
            Ok(FooPre { instance_pre, f })
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
            let f = *_instance
                .get_typed_func::<(), ((T, U, R),)>(&mut store, &self.f)?
                .func();
            Ok(Foo { f })
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
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::i::Host + Send,
        {
            foo::foo::i::add_to_linker(linker, get)?;
            Ok(())
        }
        pub async fn call_f<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
        ) -> wasmtime::Result<(T, U, R)>
        where
            <S as wasmtime::AsContext>::Data: Send,
        {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(), ((T, U, R),)>::new_unchecked(self.f)
            };
            let (ret0,) = callee.call_async(store.as_context_mut(), ()).await?;
            callee.post_return_async(store.as_context_mut()).await?;
            Ok(ret0)
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod i {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type T = u16;
            const _: () = {
                assert!(2 == < T as wasmtime::component::ComponentType >::SIZE32);
                assert!(2 == < T as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {}
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Host + Send + Sync + Copy + 'static {
                type Host: Host + Send;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host + Send,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()>
            where
                T: Send,
            {
                let mut inst = linker.instance("foo:foo/i")?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host + Send,
                T: Send,
            {
                add_to_linker_get_host(linker, get)
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
    }
}
