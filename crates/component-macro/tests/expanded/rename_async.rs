/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `neptune`.
///
/// This structure is created through [`NeptunePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct NeptunePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `neptune`.
///
/// This structure is created through either
/// [`Neptune::instantiate_async`] or by first creating
/// a [`NeptunePre`] followed by using
/// [`NeptunePre::instantiate_async`].
pub struct Neptune {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> NeptunePre<_T> {
        /// Creates a new copy of `NeptunePre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            Ok(NeptunePre { instance_pre })
        }
        /// Instantiates a new instance of [`Neptune`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Neptune>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            Ok(Neptune {})
        }
    }
    impl Neptune {
        /// Convenience wrapper around [`NeptunePre::new`] and
        /// [`NeptunePre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Neptune>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            NeptunePre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::green::Host + foo::foo::red::Host + Send,
        {
            foo::foo::green::add_to_linker(linker, get)?;
            foo::foo::red::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod green {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type Thing = i32;
            const _: () = {
                assert!(4 == < Thing as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Thing as wasmtime::component::ComponentType >::ALIGN32);
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
                let mut inst = linker.instance("foo:foo/green")?;
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
        #[allow(clippy::all)]
        pub mod red {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type Thing = super::super::super::foo::foo::green::Thing;
            const _: () = {
                assert!(4 == < Thing as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Thing as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn foo(&mut self) -> Thing;
            }
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
                let mut inst = linker.instance("foo:foo/red")?;
                inst.func_wrap_async(
                    "foo",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::foo(host).await;
                        Ok((r,))
                    }),
                )?;
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                async fn foo(&mut self) -> Thing {
                    Host::foo(*self).await
                }
            }
        }
    }
}
