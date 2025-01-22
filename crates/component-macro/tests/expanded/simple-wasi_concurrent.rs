/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `wasi`.
///
/// This structure is created through [`WasiPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Wasi`] as well.
pub struct WasiPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: WasiIndices,
}
impl<T> Clone for WasiPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> WasiPre<_T> {
    /// Creates a new copy of `WasiPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = WasiIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
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
        _T: Send + 'static,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `wasi`.
///
/// This is an implementation detail of [`WasiPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Wasi`] as well.
#[derive(Clone)]
pub struct WasiIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `wasi`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Wasi::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`WasiPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`WasiPre::instantiate_async`] to
///   create a [`Wasi`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Wasi::new`].
///
/// * You can also access the guts of instantiation through
///   [`WasiIndices::new_instance`] followed
///   by [`WasiIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Wasi {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl WasiIndices {
        /// Creates a new copy of `WasiIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            Ok(WasiIndices {})
        }
        /// Creates a new instance of [`WasiIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`Wasi`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`Wasi`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            Ok(WasiIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Wasi`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Wasi> {
            let _instance = instance;
            Ok(Wasi {})
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
            _T: Send + 'static,
        {
            let pre = linker.instantiate_pre(component)?;
            WasiPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`WasiIndices::new_instance`] and
        /// [`WasiIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Wasi> {
            let indices = WasiIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send + foo::foo::wasi_filesystem::Host<Data = T>
                + foo::foo::wall_clock::Host + 'static,
            U: Send + foo::foo::wasi_filesystem::Host<Data = T>
                + foo::foo::wall_clock::Host,
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
            use wasmtime::component::__internal::{anyhow, Box};
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
            pub trait Host {
                type Data;
                fn create_directory_at(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Result<(), Errno> + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn stat(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Result<DescriptorStat, Errno> + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
            }
            pub trait GetHost<
                T,
                D,
            >: Fn(T) -> <Self as GetHost<T, D>>::Host + Send + Sync + Copy + 'static {
                type Host: Host<Data = D> + Send;
            }
            impl<F, T, D, O> GetHost<T, D> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host<Data = D> + Send,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<
                T,
                G: for<'a> GetHost<&'a mut T, T, Host: Host<Data = T> + Send>,
            >(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: G,
            ) -> wasmtime::Result<()>
            where
                T: Send + 'static,
            {
                let mut inst = linker.instance("foo:foo/wasi-filesystem")?;
                inst.func_wrap_concurrent(
                    "create-directory-at",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::create_directory_at(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(Result<(), Errno>,)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(Result<(), Errno>,)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "stat",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::stat(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<
                                            (Result<DescriptorStat, Errno>,),
                                        > + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<
                                                    (Result<DescriptorStat, Errno>,),
                                                > + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host<Data = T> + Send,
                T: Send + 'static,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host> Host for &mut _T {
                type Data = _T::Data;
                fn create_directory_at(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Result<(), Errno> + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::create_directory_at(store)
                }
                fn stat(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Result<DescriptorStat, Errno> + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::stat(store)
                }
            }
        }
        #[allow(clippy::all)]
        pub mod wall_clock {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            pub trait Host {}
            pub trait GetHost<
                T,
                D,
            >: Fn(T) -> <Self as GetHost<T, D>>::Host + Send + Sync + Copy + 'static {
                type Host: Host + Send;
            }
            impl<F, T, D, O> GetHost<T, D> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host + Send,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<
                T,
                G: for<'a> GetHost<&'a mut T, T, Host: Host + Send>,
            >(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: G,
            ) -> wasmtime::Result<()>
            where
                T: Send + 'static,
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
                T: Send + 'static,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {}
        }
    }
}
