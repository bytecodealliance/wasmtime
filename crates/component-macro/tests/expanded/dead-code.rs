/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `imports`.
///
/// This structure is created through [`ImportsPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct ImportsPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `imports`.
///
/// This structure is created through either
/// [`Imports::instantiate`] or by first creating
/// a [`ImportsPre`] followed by using
/// [`ImportsPre::instantiate`].
pub struct Imports {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> ImportsPre<_T> {
        /// Creates a new copy of `ImportsPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            Ok(ImportsPre { instance_pre })
        }
        /// Instantiates a new instance of [`Imports`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub fn instantiate(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Imports> {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate(&mut store)?;
            Ok(Imports {})
        }
    }
    impl Imports {
        /// Convenience wrapper around [`ImportsPre::new`] and
        /// [`ImportsPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Imports> {
            let pre = linker.instantiate_pre(component)?;
            ImportsPre::new(pre)?.instantiate(store)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: a::b::interface_with_live_type::Host
                + a::b::interface_with_dead_type::Host,
        {
            a::b::interface_with_live_type::add_to_linker(linker, get)?;
            a::b::interface_with_dead_type::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
pub mod a {
    pub mod b {
        #[allow(clippy::all)]
        pub mod interface_with_live_type {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct LiveType {
                #[component(name = "a")]
                pub a: u32,
            }
            impl core::fmt::Debug for LiveType {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("LiveType").field("a", &self.a).finish()
                }
            }
            const _: () = {
                assert!(4 == < LiveType as wasmtime::component::ComponentType >::SIZE32);
                assert!(
                    4 == < LiveType as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub trait Host {
                fn f(&mut self) -> LiveType;
            }
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Host + Send + Sync + Copy + 'static {
                type Host: Host;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("a:b/interface-with-live-type")?;
                inst.func_wrap(
                    "f",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f(host);
                        Ok((r,))
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {
                fn f(&mut self) -> LiveType {
                    Host::f(*self)
                }
            }
        }
        #[allow(clippy::all)]
        pub mod interface_with_dead_type {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {}
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Host + Send + Sync + Copy + 'static {
                type Host: Host;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("a:b/interface-with-dead-type")?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {}
        }
    }
}
