/// Link-time configurations.
#[derive(Clone, Debug, Default)]
pub struct LinkOptions {
    pub(crate) experimental: bool,
}
impl LinkOptions {
    /// Enable members marked as `@unstable(feature = experimental)`
    pub fn experimental(&mut self, enabled: bool) -> &mut Self {
        self.experimental = enabled;
        self
    }
}
/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `w`.
///
/// This structure is created through [`WPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`W`] as well.
pub struct WPre<T: 'static> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: WIndices,
}
impl<T: 'static> Clone for WPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T: 'static> WPre<_T> {
    /// Creates a new copy of `WPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = WIndices::new(&instance_pre)?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`W`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<W> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
        self.indices.load(&mut store, &instance)
    }
}
impl<_T: Send + 'static> WPre<_T> {
    /// Same as [`Self::instantiate`], except with `async`.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<W> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `w`.
///
/// This is an implementation detail of [`WPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`W`] as well.
#[derive(Clone)]
pub struct WIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `w`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`W::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`WPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`WPre::instantiate`] to
///   create a [`W`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`W::new`].
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct W {}
const _: () = {
    impl WIndices {
        /// Creates a new copy of `WIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new<_T>(
            _instance_pre: &wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = _instance_pre.component();
            let _instance_type = _instance_pre.instance_type();
            Ok(WIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`W`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<W> {
            let _ = &mut store;
            let _instance = instance;
            Ok(W {})
        }
    }
    impl W {
        /// Convenience wrapper around [`WPre::new`] and
        /// [`WPre::instantiate`].
        pub fn instantiate<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<W> {
            let pre = linker.instantiate_pre(component)?;
            WPre::new(pre)?.instantiate(store)
        }
        /// Convenience wrapper around [`WIndices::new`] and
        /// [`WIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<W> {
            let indices = WIndices::new(&instance.instance_pre(&store))?;
            indices.load(&mut store, instance)
        }
        /// Convenience wrapper around [`WPre::new`] and
        /// [`WPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<W>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            WPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, D>(
            linker: &mut wasmtime::component::Linker<T>,
            options: &LinkOptions,
            host_getter: fn(&mut T) -> D::Data<'_>,
        ) -> wasmtime::Result<()>
        where
            D: foo::foo::store::HostWithStore<T> + foo::foo::store::HostWithStore<T>
                + foo::foo::cache::HostWithStore<T> + foo::foo::cache::HostWithStore<T>
                + Send,
            for<'a> D::Data<
                'a,
            >: foo::foo::store::Host + foo::foo::store::Host + foo::foo::cache::Host
                + foo::foo::cache::Host + Send,
            T: 'static + Send,
        {
            foo::foo::store::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("a")?, host_getter)?;
            if options.experimental {
                foo::foo::store::add_to_linker_instance::<
                    T,
                    D,
                >(&mut linker.instance("b")?, host_getter)?;
            }
            if options.experimental {
                foo::foo::cache::add_to_linker_instance::<
                    T,
                    D,
                >(&mut linker.instance("c")?, host_getter)?;
            }
            foo::foo::cache::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("d")?, host_getter)?;
            Ok(())
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod store {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {}
            impl<H: ?Sized, T> HostWithStore<T> for H
            where
                H: wasmtime::component::HasData + Send,
            {}
            pub trait Host: Send {
                fn get(&mut self) -> impl ::core::future::Future<Output = u32> + Send;
            }
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                fn get(&mut self) -> impl ::core::future::Future<Output = u32> + Send {
                    async move { Host::get(*self).await }
                }
            }
            pub fn add_to_linker_instance<T, D>(
                inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                inst.func_wrap_async(
                    "get",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "store", function = "get",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::get(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, D>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                let mut inst = linker.instance("foo:foo/store")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
        #[allow(clippy::all)]
        pub mod cache {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {}
            impl<H: ?Sized, T> HostWithStore<T> for H
            where
                H: wasmtime::component::HasData + Send,
            {}
            pub trait Host: Send {
                fn get(&mut self) -> impl ::core::future::Future<Output = u32> + Send;
            }
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                fn get(&mut self) -> impl ::core::future::Future<Output = u32> + Send {
                    async move { Host::get(*self).await }
                }
            }
            pub fn add_to_linker_instance<T, D>(
                inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                inst.func_wrap_async(
                    "get",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "cache", function = "get",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::get(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, D>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                let mut inst = linker.instance("foo:foo/cache")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
    }
}
