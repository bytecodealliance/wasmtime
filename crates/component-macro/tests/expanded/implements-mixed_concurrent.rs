/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `mixed`.
///
/// This structure is created through [`MixedPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Mixed`] as well.
pub struct MixedPre<T: 'static> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: MixedIndices,
}
impl<T: 'static> Clone for MixedPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T: 'static> MixedPre<_T> {
    /// Creates a new copy of `MixedPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = MixedIndices::new(&instance_pre)?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`Mixed`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Mixed> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
        self.indices.load(&mut store, &instance)
    }
}
impl<_T: Send + 'static> MixedPre<_T> {
    /// Same as [`Self::instantiate`], except with `async`.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Mixed> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `mixed`.
///
/// This is an implementation detail of [`MixedPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Mixed`] as well.
#[derive(Clone)]
pub struct MixedIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `mixed`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Mixed::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`MixedPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`MixedPre::instantiate`] to
///   create a [`Mixed`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Mixed::new`].
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Mixed {}
const _: () = {
    impl MixedIndices {
        /// Creates a new copy of `MixedIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new<_T>(
            _instance_pre: &wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = _instance_pre.component();
            let _instance_type = _instance_pre.instance_type();
            Ok(MixedIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Mixed`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Mixed> {
            let _ = &mut store;
            let _instance = instance;
            Ok(Mixed {})
        }
    }
    impl Mixed {
        /// Convenience wrapper around [`MixedPre::new`] and
        /// [`MixedPre::instantiate`].
        pub fn instantiate<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Mixed> {
            let pre = linker.instantiate_pre(component)?;
            MixedPre::new(pre)?.instantiate(store)
        }
        /// Convenience wrapper around [`MixedIndices::new`] and
        /// [`MixedIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Mixed> {
            let indices = MixedIndices::new(&instance.instance_pre(&store))?;
            indices.load(&mut store, instance)
        }
        /// Convenience wrapper around [`MixedPre::new`] and
        /// [`MixedPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Mixed>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            MixedPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, D>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: fn(&mut T) -> D::Data<'_>,
        ) -> wasmtime::Result<()>
        where
            D: foo::foo::store::HostWithStore<T> + foo::foo::cache::HostWithStore<T>
                + foo::foo::types::HostWithStore<T>
                + foo::foo::consumer::HostWithStore<T>
                + foo::foo::store::HostWithStore<T> + foo::foo::cache::HostWithStore<T>
                + foo::foo::types::HostWithStore<T> + Send,
            for<'a> D::Data<
                'a,
            >: foo::foo::store::Host + foo::foo::cache::Host + foo::foo::types::Host
                + foo::foo::consumer::Host + foo::foo::store::Host
                + foo::foo::cache::Host + foo::foo::types::Host + Send,
            T: 'static + Send,
        {
            foo::foo::store::add_to_linker::<T, D>(linker, host_getter)?;
            foo::foo::cache::add_to_linker::<T, D>(linker, host_getter)?;
            foo::foo::types::add_to_linker::<T, D>(linker, host_getter)?;
            foo::foo::consumer::add_to_linker::<T, D>(linker, host_getter)?;
            foo::foo::store::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("a")?, host_getter)?;
            foo::foo::cache::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("b")?, host_getter)?;
            foo::foo::types::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("c")?, host_getter)?;
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
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
                fn get(
                    host: wasmtime::component::Access<T, Self>,
                    key: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<
                    Output = Option<wasmtime::component::__internal::String>,
                > + Send;
            }
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
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
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::get(host, arg0).await;
                            Ok((r,))
                        })
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
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
                fn get(
                    host: wasmtime::component::Access<T, Self>,
                    key: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<
                    Output = Option<wasmtime::component::__internal::String>,
                > + Send;
            }
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
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
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::get(host, arg0).await;
                            Ok((r,))
                        })
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
        #[allow(clippy::all)]
        pub mod types {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub enum R {}
            pub trait HostRWithStore<T>: wasmtime::component::HasData {
                fn drop(
                    accessor: &wasmtime::component::Accessor<T, Self>,
                    rep: wasmtime::component::Resource<R>,
                ) -> impl ::core::future::Future<Output = wasmtime::Result<()>> + Send
                where
                    Self: Sized;
            }
            pub trait HostR {}
            impl<_T: HostR + ?Sized + Send> HostR for &mut _T {}
            pub trait HostWithStore<
                T,
            >: wasmtime::component::HasData + HostRWithStore<T> + Send {
                fn make(
                    host: wasmtime::component::Access<T, Self>,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::Resource<R>,
                > + Send;
            }
            pub trait Host: HostR + Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
            pub fn add_to_linker_instance<T, D>(
                inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                inst.resource_concurrent(
                    "r",
                    wasmtime::component::ResourceType::host::<R>(),
                    move |caller: &wasmtime::component::Accessor<T>, rep| {
                        wasmtime::component::__internal::Box::pin(async move {
                            let accessor = &caller.with_getter::<D>(host_getter);
                            wasmtime::ToWasmtimeResult::to_wasmtime_result(
                                HostRWithStore::<
                                    T,
                                >::drop(
                                        accessor,
                                        wasmtime::component::Resource::new_own(rep),
                                    )
                                    .await,
                            )
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "make",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::make(host).await;
                            Ok((r,))
                        })
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
                let mut inst = linker.instance("foo:foo/types")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
        #[allow(clippy::all)]
        pub mod consumer {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub type R = super::super::super::foo::foo::types::R;
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
                fn take(
                    host: wasmtime::component::Access<T, Self>,
                    x: wasmtime::component::Resource<R>,
                ) -> impl ::core::future::Future<Output = ()> + Send;
            }
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
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
                    "take",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::Resource<R>,)|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::take(host, arg0).await;
                            Ok(r)
                        })
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
                let mut inst = linker.instance("foo:foo/consumer")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
    }
}
