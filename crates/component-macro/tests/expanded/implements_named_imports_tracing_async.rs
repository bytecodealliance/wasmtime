#[doc(hidden)]
pub use u32 as __named_import_name0;
/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `cache`.
///
/// This structure is created through [`CachePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Cache`] as well.
pub struct CachePre<T: 'static> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: CacheIndices,
}
impl<T: 'static> Clone for CachePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T: 'static> CachePre<_T> {
    /// Creates a new copy of `CachePre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = CacheIndices::new(&instance_pre)?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`Cache`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Cache> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
        self.indices.load(&mut store, &instance)
    }
}
impl<_T: Send + 'static> CachePre<_T> {
    /// Same as [`Self::instantiate`], except with `async`.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Cache> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `cache`.
///
/// This is an implementation detail of [`CachePre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Cache`] as well.
#[derive(Clone)]
pub struct CacheIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `cache`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Cache::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`CachePre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`CachePre::instantiate`] to
///   create a [`Cache`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Cache::new`].
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Cache {}
const _: () = {
    impl CacheIndices {
        /// Creates a new copy of `CacheIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new<_T>(
            _instance_pre: &wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = _instance_pre.component();
            let _instance_type = _instance_pre.instance_type();
            Ok(CacheIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Cache`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Cache> {
            let _ = &mut store;
            let _instance = instance;
            Ok(Cache {})
        }
    }
    impl Cache {
        /// Convenience wrapper around [`CachePre::new`] and
        /// [`CachePre::instantiate`].
        pub fn instantiate<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Cache> {
            let pre = linker.instantiate_pre(component)?;
            CachePre::new(pre)?.instantiate(store)
        }
        /// Convenience wrapper around [`CacheIndices::new`] and
        /// [`CacheIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Cache> {
            let indices = CacheIndices::new(&instance.instance_pre(&store))?;
            indices.load(&mut store, instance)
        }
        /// Convenience wrapper around [`CachePre::new`] and
        /// [`CachePre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Cache>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            CachePre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, D>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: fn(&mut T) -> D::Data<'_>,
        ) -> wasmtime::Result<()>
        where
            D: foo::foo::store::HostWithStore<T> + foo::foo::store::HostWithStore<T>
                + Send,
            for<'a> D::Data<'a>: foo::foo::store::Host + foo::foo::store::Host + Send,
            T: 'static + Send,
        {
            foo::foo::store::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("hot-cache")?, host_getter)?;
            foo::foo::store::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("durable")?, host_getter)?;
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
                fn get(
                    &mut self,
                    key: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<
                    Output = Option<wasmtime::component::__internal::String>,
                > + Send;
                fn set(
                    &mut self,
                    key: wasmtime::component::__internal::String,
                    value: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<Output = ()> + Send;
            }
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                fn get(
                    &mut self,
                    key: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<
                    Output = Option<wasmtime::component::__internal::String>,
                > + Send {
                    async move { Host::get(*self, key).await }
                }
                fn set(
                    &mut self,
                    key: wasmtime::component::__internal::String,
                    value: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<Output = ()> + Send {
                    async move { Host::set(*self, key, value).await }
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
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::String,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "store", function = "get",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, key = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::get(host, arg0).await;
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
                inst.func_wrap_async(
                    "set",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                        ): (
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::String,
                        )|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "store", function = "set",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, key = tracing::field::debug(& arg0),
                                    value = tracing::field::debug(& arg1), "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::set(host, arg0, arg1).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
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
    }
}
pub mod named_imports {
    pub mod foo {
        pub mod foo {
            pub mod store {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::Box;
                pub trait HostWithStore<T>: wasmtime::component::HasData + Send {}
                impl<H: ?Sized, T> HostWithStore<T> for H
                where
                    H: wasmtime::component::HasData + Send,
                {}
                pub trait Host: Send {
                    fn get(
                        &mut self,
                        id: super::super::super::super::__named_import_name0,
                        key: wasmtime::component::__internal::String,
                    ) -> impl ::core::future::Future<
                        Output = Option<wasmtime::component::__internal::String>,
                    > + Send;
                    fn set(
                        &mut self,
                        id: super::super::super::super::__named_import_name0,
                        key: wasmtime::component::__internal::String,
                        value: wasmtime::component::__internal::String,
                    ) -> impl ::core::future::Future<Output = ()> + Send;
                }
                impl<_T: Host + ?Sized + Send> Host for &mut _T {
                    fn get(
                        &mut self,
                        id: super::super::super::super::__named_import_name0,
                        key: wasmtime::component::__internal::String,
                    ) -> impl ::core::future::Future<
                        Output = Option<wasmtime::component::__internal::String>,
                    > + Send {
                        async move { Host::get(*self, id, key).await }
                    }
                    fn set(
                        &mut self,
                        id: super::super::super::super::__named_import_name0,
                        key: wasmtime::component::__internal::String,
                        value: wasmtime::component::__internal::String,
                    ) -> impl ::core::future::Future<Output = ()> + Send {
                        async move { Host::set(*self, id, key, value).await }
                    }
                }
                pub fn add_to_linker_instance<T, D>(
                    inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                    name: &str,
                    id: super::super::super::super::__named_import_name0,
                    host_getter: fn(&mut T) -> D::Data<'_>,
                ) -> wasmtime::Result<()>
                where
                    D: HostWithStore<T>,
                    for<'a> D::Data<'a>: Host,
                    T: 'static + Send,
                {
                    let _ = name;
                    inst.func_wrap_async(
                        "get",
                        {
                            let id = id.clone();
                            let name = wasmtime::component::__internal::String::from(
                                name,
                            );
                            move |
                                mut caller: wasmtime::StoreContextMut<'_, T>,
                                (arg0,): (wasmtime::component::__internal::String,)|
                            {
                                use tracing::Instrument;
                                let span = tracing::span!(
                                    tracing::Level::TRACE, "wit-bindgen import", module =
                                    "store", label = name.as_str(), function = "get",
                                );
                                let id = id.clone();
                                wasmtime::component::__internal::Box::new(
                                    async move {
                                        tracing::event!(
                                            tracing::Level::TRACE, key = tracing::field::debug(& arg0),
                                            "call"
                                        );
                                        let host = &mut host_getter(caller.data_mut());
                                        let r = Host::get(host, id, arg0).await;
                                        tracing::event!(
                                            tracing::Level::TRACE, result = tracing::field::debug(& r),
                                            "return"
                                        );
                                        Ok((r,))
                                    }
                                        .instrument(span),
                                )
                            }
                        },
                    )?;
                    inst.func_wrap_async(
                        "set",
                        {
                            let id = id.clone();
                            let name = wasmtime::component::__internal::String::from(
                                name,
                            );
                            move |
                                mut caller: wasmtime::StoreContextMut<'_, T>,
                                (
                                    arg0,
                                    arg1,
                                ): (
                                    wasmtime::component::__internal::String,
                                    wasmtime::component::__internal::String,
                                )|
                            {
                                use tracing::Instrument;
                                let span = tracing::span!(
                                    tracing::Level::TRACE, "wit-bindgen import", module =
                                    "store", label = name.as_str(), function = "set",
                                );
                                let id = id.clone();
                                wasmtime::component::__internal::Box::new(
                                    async move {
                                        tracing::event!(
                                            tracing::Level::TRACE, key = tracing::field::debug(& arg0),
                                            value = tracing::field::debug(& arg1), "call"
                                        );
                                        let host = &mut host_getter(caller.data_mut());
                                        let r = Host::set(host, id, arg0, arg1).await;
                                        tracing::event!(
                                            tracing::Level::TRACE, result = tracing::field::debug(& r),
                                            "return"
                                        );
                                        Ok(r)
                                    }
                                        .instrument(span),
                                )
                            }
                        },
                    )?;
                    Ok(())
                }
                pub fn add_to_linker<T, D>(
                    linker: &mut wasmtime::component::Linker<T>,
                    component: &wasmtime::component::Component,
                    mut lookup: impl FnMut(
                        &str,
                        &wasmtime::component::types::ComponentExtern<'_>,
                    ) -> wasmtime::Result<
                            ::core::option::Option<
                                super::super::super::super::__named_import_name0,
                            >,
                        >,
                    host_getter: fn(&mut T) -> D::Data<'_>,
                ) -> wasmtime::Result<()>
                where
                    D: HostWithStore<T>,
                    for<'a> D::Data<'a>: Host,
                    T: 'static + Send,
                {
                    let engine = linker.engine().clone();
                    let component_ty = component.component_type();
                    let mut matched = wasmtime::component::__internal::Vec::new();
                    for (name, item) in component_ty.imports(&engine) {
                        if !item.is_implements("foo:foo/store") {
                            continue;
                        }
                        if let Some(id) = lookup(name, &item)? {
                            matched.push((name, id));
                        }
                    }
                    for (name, id) in matched {
                        let mut inst = linker.instance(name)?;
                        add_to_linker_instance::<
                            T,
                            D,
                        >(&mut inst, name, id, host_getter)?;
                    }
                    Ok(())
                }
            }
        }
    }
}
