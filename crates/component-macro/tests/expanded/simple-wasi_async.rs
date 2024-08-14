/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `wasi`.
///
/// This structure is created through [`WasiPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct WasiPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
}
impl<T> Clone for WasiPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `wasi`.
///
/// This structure is created through either
/// [`Wasi::instantiate_async`] or by first creating
/// a [`WasiPre`] followed by using
/// [`WasiPre::instantiate_async`].
pub struct Wasi {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> WasiPre<_T> {
        /// Creates a new copy of `WasiPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            Ok(WasiPre { instance_pre })
        }
        /// Instantiates a new instance of [`Wasi`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Wasi>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            Ok(Wasi {})
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl Wasi {
        /// Convenience wrapper around [`WasiPre::new`] and
        /// [`WasiPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Wasi>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            WasiPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::wasi_filesystem::Host + foo::foo::wall_clock::Host + Send,
        {
            foo::foo::wasi_filesystem::add_to_linker(linker, get)?;
            foo::foo::wall_clock::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod wasi_filesystem {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct DescriptorStat {}
            impl core::fmt::Debug for DescriptorStat {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("DescriptorStat").finish()
                }
            }
            const _: () = {
                assert!(
                    0 == < DescriptorStat as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    1 == < DescriptorStat as wasmtime::component::ComponentType
                    >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(enum)]
            #[derive(Clone, Copy, Eq, PartialEq)]
            #[repr(u8)]
            pub enum Errno {
                #[component(name = "e")]
                E,
            }
            impl Errno {
                pub fn name(&self) -> &'static str {
                    match self {
                        Errno::E => "e",
                    }
                }
                pub fn message(&self) -> &'static str {
                    match self {
                        Errno::E => "",
                    }
                }
            }
            impl core::fmt::Debug for Errno {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Errno")
                        .field("code", &(*self as i32))
                        .field("name", &self.name())
                        .field("message", &self.message())
                        .finish()
                }
            }
            impl core::fmt::Display for Errno {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{} (error {})", self.name(), * self as i32)
                }
            }
            impl std::error::Error for Errno {}
            const _: () = {
                assert!(1 == < Errno as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Errno as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn create_directory_at(&mut self) -> Result<(), Errno>;
                async fn stat(&mut self) -> Result<DescriptorStat, Errno>;
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
                let mut inst = linker.instance("foo:foo/wasi-filesystem")?;
                inst.func_wrap_async(
                    "create-directory-at",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::create_directory_at(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "stat",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::stat(host).await;
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
                async fn create_directory_at(&mut self) -> Result<(), Errno> {
                    Host::create_directory_at(*self).await
                }
                async fn stat(&mut self) -> Result<DescriptorStat, Errno> {
                    Host::stat(*self).await
                }
            }
        }
        #[allow(clippy::all)]
        pub mod wall_clock {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
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
                let mut inst = linker.instance("foo:foo/wall-clock")?;
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
