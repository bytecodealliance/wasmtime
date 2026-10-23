/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-maps`.
///
/// This structure is created through [`TheMapsPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`TheMaps`] as well.
pub struct TheMapsPre<T: 'static> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: TheMapsIndices,
}
impl<T: 'static> Clone for TheMapsPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T: 'static> TheMapsPre<_T> {
    /// Creates a new copy of `TheMapsPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = TheMapsIndices::new(&instance_pre)?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`TheMaps`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<TheMaps> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
        self.indices.load(&mut store, &instance)
    }
}
impl<_T: Send + 'static> TheMapsPre<_T> {
    /// Same as [`Self::instantiate`], except with `async`.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<TheMaps> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `the-maps`.
///
/// This is an implementation detail of [`TheMapsPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`TheMaps`] as well.
#[derive(Clone)]
pub struct TheMapsIndices {
    interface0: exports::foo::foo::maps::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-maps`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`TheMaps::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`TheMapsPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`TheMapsPre::instantiate`] to
///   create a [`TheMaps`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`TheMaps::new`].
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct TheMaps {
    interface0: exports::foo::foo::maps::Guest,
}
const _: () = {
    impl TheMapsIndices {
        /// Creates a new copy of `TheMapsIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new<_T>(
            _instance_pre: &wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = _instance_pre.component();
            let _instance_type = _instance_pre.instance_type();
            let interface0 = exports::foo::foo::maps::GuestIndices::new(_instance_pre)?;
            Ok(TheMapsIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`TheMaps`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheMaps> {
            let _ = &mut store;
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheMaps { interface0 })
        }
    }
    impl TheMaps {
        /// Convenience wrapper around [`TheMapsPre::new`] and
        /// [`TheMapsPre::instantiate`].
        pub fn instantiate<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheMaps> {
            let pre = linker.instantiate_pre(component)?;
            TheMapsPre::new(pre)?.instantiate(store)
        }
        /// Convenience wrapper around [`TheMapsIndices::new`] and
        /// [`TheMapsIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheMaps> {
            let indices = TheMapsIndices::new(&instance.instance_pre(&store))?;
            indices.load(&mut store, instance)
        }
        /// Convenience wrapper around [`TheMapsPre::new`] and
        /// [`TheMapsPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheMaps>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            TheMapsPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, D>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: fn(&mut T) -> D::Data<'_>,
        ) -> wasmtime::Result<()>
        where
            D: foo::foo::maps::HostWithStore<T> + Send,
            for<'a> D::Data<'a>: foo::foo::maps::Host + Send,
            T: 'static + Send,
        {
            foo::foo::maps::add_to_linker::<T, D>(linker, host_getter)?;
            Ok(())
        }
        pub fn foo_foo_maps(&self) -> &exports::foo::foo::maps::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod maps {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub enum Thing {}
            pub trait HostThingWithStore<T>: wasmtime::component::HasData {
                fn drop(
                    accessor: &wasmtime::component::Accessor<T, Self>,
                    rep: wasmtime::component::Resource<Thing>,
                ) -> impl ::core::future::Future<Output = wasmtime::Result<()>> + Send
                where
                    Self: Sized;
            }
            pub trait HostThing {}
            impl<_T: HostThing + ?Sized + Send> HostThing for &mut _T {}
            pub type NamesById = wasmtime::component::__internal::BTreeMap<
                u32,
                wasmtime::component::__internal::String,
            >;
            const _: () = {
                assert!(
                    8 == < NamesById as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < NamesById as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub type IdsByName = wasmtime::component::__internal::BTreeMap<
                wasmtime::component::__internal::String,
                u32,
            >;
            const _: () = {
                assert!(
                    8 == < IdsByName as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < IdsByName as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct LabeledEntry {
                #[component(name = "label")]
                pub label: wasmtime::component::__internal::String,
                #[component(name = "values")]
                pub values: wasmtime::component::__internal::BTreeMap<
                    wasmtime::component::__internal::String,
                    u32,
                >,
            }
            impl ::core::fmt::Debug for LabeledEntry {
                fn fmt(
                    &self,
                    f: &mut ::core::fmt::Formatter<'_>,
                ) -> ::core::fmt::Result {
                    f.debug_struct("LabeledEntry")
                        .field("label", &self.label)
                        .field("values", &self.values)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    16 == < LabeledEntry as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < LabeledEntry as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum MapOrString {
                #[component(name = "as-map")]
                AsMap(
                    wasmtime::component::__internal::BTreeMap<
                        u32,
                        wasmtime::component::__internal::String,
                    >,
                ),
                #[component(name = "as-string")]
                AsString(wasmtime::component::__internal::String),
            }
            impl ::core::fmt::Debug for MapOrString {
                fn fmt(
                    &self,
                    f: &mut ::core::fmt::Formatter<'_>,
                ) -> ::core::fmt::Result {
                    match self {
                        MapOrString::AsMap(e) => {
                            f.debug_tuple("MapOrString::AsMap").field(e).finish()
                        }
                        MapOrString::AsString(e) => {
                            f.debug_tuple("MapOrString::AsString").field(e).finish()
                        }
                    }
                }
            }
            const _: () = {
                assert!(
                    12 == < MapOrString as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < MapOrString as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub trait HostWithStore<
                T,
            >: wasmtime::component::HasData + HostThingWithStore<T> + Send {
                fn named_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: NamesById,
                ) -> impl ::core::future::Future<Output = IdsByName> + Send;
                fn bytes_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::__internal::Vec<u8>,
                    >,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::__internal::Vec<u8>,
                    >,
                > + Send;
                fn option_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: Option<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                    >,
                ) -> impl ::core::future::Future<
                    Output = Option<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                    >,
                > + Send;
                fn result_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: Result<
                        wasmtime::component::__internal::BTreeMap<
                            u32,
                            wasmtime::component::__internal::String,
                        >,
                        wasmtime::component::__internal::String,
                    >,
                ) -> impl ::core::future::Future<
                    Output = Result<
                        wasmtime::component::__internal::BTreeMap<
                            u32,
                            wasmtime::component::__internal::String,
                        >,
                        wasmtime::component::__internal::String,
                    >,
                > + Send;
                fn tuple_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: (
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                        u64,
                    ),
                ) -> impl ::core::future::Future<
                    Output = (
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                        u64,
                    ),
                > + Send;
                fn nested_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::__internal::BTreeMap<
                            u32,
                            wasmtime::component::__internal::String,
                        >,
                    >,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::__internal::BTreeMap<
                            u32,
                            wasmtime::component::__internal::String,
                        >,
                    >,
                > + Send;
                fn list_of_maps(
                    host: wasmtime::component::Access<T, Self>,
                    a: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                    >,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                    >,
                > + Send;
                fn record_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: LabeledEntry,
                ) -> impl ::core::future::Future<Output = LabeledEntry> + Send;
                fn variant_roundtrip(
                    host: wasmtime::component::Access<T, Self>,
                    a: MapOrString,
                ) -> impl ::core::future::Future<Output = MapOrString> + Send;
                fn all_key_types(
                    host: wasmtime::component::Access<T, Self>,
                    a: wasmtime::component::__internal::BTreeMap<bool, u32>,
                    b: wasmtime::component::__internal::BTreeMap<u8, u32>,
                    c: wasmtime::component::__internal::BTreeMap<i8, u32>,
                    d: wasmtime::component::__internal::BTreeMap<u16, u32>,
                    e: wasmtime::component::__internal::BTreeMap<i16, u32>,
                    f: wasmtime::component::__internal::BTreeMap<u32, u32>,
                    g: wasmtime::component::__internal::BTreeMap<i32, u32>,
                    h: wasmtime::component::__internal::BTreeMap<u64, u32>,
                    i: wasmtime::component::__internal::BTreeMap<i64, u32>,
                    j: wasmtime::component::__internal::BTreeMap<char, u32>,
                    k: wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        u32,
                    >,
                ) -> impl ::core::future::Future<Output = ()> + Send;
                fn own_values(
                    host: wasmtime::component::Access<T, Self>,
                    a: wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::Resource<Thing>,
                    >,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::Resource<Thing>,
                    >,
                > + Send;
                fn borrowed_values(
                    host: wasmtime::component::Access<T, Self>,
                    a: wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        wasmtime::component::Resource<Thing>,
                    >,
                ) -> impl ::core::future::Future<Output = ()> + Send;
            }
            pub trait Host: HostThing + Send {}
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
                    "thing",
                    wasmtime::component::ResourceType::host::<Thing>(),
                    move |caller: &wasmtime::component::Accessor<T>, rep| {
                        wasmtime::component::__internal::Box::pin(async move {
                            let accessor = &caller.with_getter::<D>(host_getter);
                            wasmtime::ToWasmtimeResult::to_wasmtime_result(
                                HostThingWithStore::<
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
                    "named-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (NamesById,)|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::named_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "bytes-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::Vec<u8>,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::bytes_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "option-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            Option<
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::option_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "result-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            Result<
                                wasmtime::component::__internal::BTreeMap<
                                    u32,
                                    wasmtime::component::__internal::String,
                                >,
                                wasmtime::component::__internal::String,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::result_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "tuple-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            (
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                                u64,
                            ),
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::tuple_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "nested-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::BTreeMap<
                                    u32,
                                    wasmtime::component::__internal::String,
                                >,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::nested_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "list-of-maps",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::list_of_maps(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "record-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (LabeledEntry,)|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::record_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "variant-roundtrip",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (MapOrString,)|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<
                                T,
                            >>::variant_roundtrip(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "all-key-types",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                            arg6,
                            arg7,
                            arg8,
                            arg9,
                            arg10,
                        ): (
                            wasmtime::component::__internal::BTreeMap<bool, u32>,
                            wasmtime::component::__internal::BTreeMap<u8, u32>,
                            wasmtime::component::__internal::BTreeMap<i8, u32>,
                            wasmtime::component::__internal::BTreeMap<u16, u32>,
                            wasmtime::component::__internal::BTreeMap<i16, u32>,
                            wasmtime::component::__internal::BTreeMap<u32, u32>,
                            wasmtime::component::__internal::BTreeMap<i32, u32>,
                            wasmtime::component::__internal::BTreeMap<u64, u32>,
                            wasmtime::component::__internal::BTreeMap<i64, u32>,
                            wasmtime::component::__internal::BTreeMap<char, u32>,
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<
                                T,
                            >>::all_key_types(
                                    host,
                                    arg0,
                                    arg1,
                                    arg2,
                                    arg3,
                                    arg4,
                                    arg5,
                                    arg6,
                                    arg7,
                                    arg8,
                                    arg9,
                                    arg10,
                                )
                                .await;
                            Ok(r)
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "own-values",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::Resource<Thing>,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::own_values(host, arg0)
                                .await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "borrowed-values",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::Resource<Thing>,
                            >,
                        )|
                    {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::borrowed_values(host, arg0)
                                .await;
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
                let mut inst = linker.instance("foo:foo/maps")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod maps {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::Box;
                pub type Thing = wasmtime::component::ResourceAny;
                pub struct GuestThing<'a> {
                    funcs: &'a Guest,
                }
                pub type NamesById = wasmtime::component::__internal::BTreeMap<
                    u32,
                    wasmtime::component::__internal::String,
                >;
                const _: () = {
                    assert!(
                        8 == < NamesById as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < NamesById as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                pub type IdsByName = wasmtime::component::__internal::BTreeMap<
                    wasmtime::component::__internal::String,
                    u32,
                >;
                const _: () = {
                    assert!(
                        8 == < IdsByName as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < IdsByName as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct LabeledEntry {
                    #[component(name = "label")]
                    pub label: wasmtime::component::__internal::String,
                    #[component(name = "values")]
                    pub values: wasmtime::component::__internal::BTreeMap<
                        wasmtime::component::__internal::String,
                        u32,
                    >,
                }
                impl ::core::fmt::Debug for LabeledEntry {
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter<'_>,
                    ) -> ::core::fmt::Result {
                        f.debug_struct("LabeledEntry")
                            .field("label", &self.label)
                            .field("values", &self.values)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        16 == < LabeledEntry as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < LabeledEntry as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone)]
                pub enum MapOrString {
                    #[component(name = "as-map")]
                    AsMap(
                        wasmtime::component::__internal::BTreeMap<
                            u32,
                            wasmtime::component::__internal::String,
                        >,
                    ),
                    #[component(name = "as-string")]
                    AsString(wasmtime::component::__internal::String),
                }
                impl ::core::fmt::Debug for MapOrString {
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter<'_>,
                    ) -> ::core::fmt::Result {
                        match self {
                            MapOrString::AsMap(e) => {
                                f.debug_tuple("MapOrString::AsMap").field(e).finish()
                            }
                            MapOrString::AsString(e) => {
                                f.debug_tuple("MapOrString::AsString").field(e).finish()
                            }
                        }
                    }
                }
                const _: () = {
                    assert!(
                        12 == < MapOrString as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < MapOrString as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(Clone)]
                pub struct Guest {
                    named_roundtrip: wasmtime::component::Func,
                    bytes_roundtrip: wasmtime::component::Func,
                    option_roundtrip: wasmtime::component::Func,
                    result_roundtrip: wasmtime::component::Func,
                    tuple_roundtrip: wasmtime::component::Func,
                    nested_roundtrip: wasmtime::component::Func,
                    list_of_maps: wasmtime::component::Func,
                    record_roundtrip: wasmtime::component::Func,
                    variant_roundtrip: wasmtime::component::Func,
                    all_key_types: wasmtime::component::Func,
                    own_values: wasmtime::component::Func,
                    borrowed_values: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    named_roundtrip: wasmtime::component::ComponentExportIndex,
                    bytes_roundtrip: wasmtime::component::ComponentExportIndex,
                    option_roundtrip: wasmtime::component::ComponentExportIndex,
                    result_roundtrip: wasmtime::component::ComponentExportIndex,
                    tuple_roundtrip: wasmtime::component::ComponentExportIndex,
                    nested_roundtrip: wasmtime::component::ComponentExportIndex,
                    list_of_maps: wasmtime::component::ComponentExportIndex,
                    record_roundtrip: wasmtime::component::ComponentExportIndex,
                    variant_roundtrip: wasmtime::component::ComponentExportIndex,
                    all_key_types: wasmtime::component::ComponentExportIndex,
                    own_values: wasmtime::component::ComponentExportIndex,
                    borrowed_values: wasmtime::component::ComponentExportIndex,
                }
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new<_T>(
                        _instance_pre: &wasmtime::component::InstancePre<_T>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance = _instance_pre
                            .component()
                            .get_export_index(None, "foo:foo/maps")
                            .ok_or_else(|| {
                                wasmtime::format_err!(
                                    "no exported instance named `foo:foo/maps`"
                                )
                            })?;
                        let mut lookup = move |name: &str| {
                            _instance_pre
                                .component()
                                .get_export_index(Some(&instance), name)
                                .ok_or_else(|| {
                                    wasmtime::format_err!(
                                        "instance export `foo:foo/maps` does \
                  not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let named_roundtrip = lookup("named-roundtrip")?;
                        let bytes_roundtrip = lookup("bytes-roundtrip")?;
                        let option_roundtrip = lookup("option-roundtrip")?;
                        let result_roundtrip = lookup("result-roundtrip")?;
                        let tuple_roundtrip = lookup("tuple-roundtrip")?;
                        let nested_roundtrip = lookup("nested-roundtrip")?;
                        let list_of_maps = lookup("list-of-maps")?;
                        let record_roundtrip = lookup("record-roundtrip")?;
                        let variant_roundtrip = lookup("variant-roundtrip")?;
                        let all_key_types = lookup("all-key-types")?;
                        let own_values = lookup("own-values")?;
                        let borrowed_values = lookup("borrowed-values")?;
                        Ok(GuestIndices {
                            named_roundtrip,
                            bytes_roundtrip,
                            option_roundtrip,
                            result_roundtrip,
                            tuple_roundtrip,
                            nested_roundtrip,
                            list_of_maps,
                            record_roundtrip,
                            variant_roundtrip,
                            all_key_types,
                            own_values,
                            borrowed_values,
                        })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let _instance = instance;
                        let _instance_pre = _instance.instance_pre(&store);
                        let _instance_type = _instance_pre.instance_type();
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let named_roundtrip = *_instance
                            .get_typed_func::<
                                (&NamesById,),
                                (IdsByName,),
                            >(&mut store, &self.named_roundtrip)?
                            .func();
                        let bytes_roundtrip = *_instance
                            .get_typed_func::<
                                (
                                    &wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::Vec<u8>,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::Vec<u8>,
                                    >,
                                ),
                            >(&mut store, &self.bytes_roundtrip)?
                            .func();
                        let option_roundtrip = *_instance
                            .get_typed_func::<
                                (
                                    Option<
                                        &wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                                (
                                    Option<
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                            >(&mut store, &self.option_roundtrip)?
                            .func();
                        let result_roundtrip = *_instance
                            .get_typed_func::<
                                (
                                    Result<
                                        &wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                        &str,
                                    >,
                                ),
                                (
                                    Result<
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >(&mut store, &self.result_roundtrip)?
                            .func();
                        let tuple_roundtrip = *_instance
                            .get_typed_func::<
                                (
                                    (
                                        &wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                        u64,
                                    ),
                                ),
                                (
                                    (
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                        u64,
                                    ),
                                ),
                            >(&mut store, &self.tuple_roundtrip)?
                            .func();
                        let nested_roundtrip = *_instance
                            .get_typed_func::<
                                (
                                    &wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                    >,
                                ),
                            >(&mut store, &self.nested_roundtrip)?
                            .func();
                        let list_of_maps = *_instance
                            .get_typed_func::<
                                (
                                    &[wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        u32,
                                    >],
                                ),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                            >(&mut store, &self.list_of_maps)?
                            .func();
                        let record_roundtrip = *_instance
                            .get_typed_func::<
                                (&LabeledEntry,),
                                (LabeledEntry,),
                            >(&mut store, &self.record_roundtrip)?
                            .func();
                        let variant_roundtrip = *_instance
                            .get_typed_func::<
                                (&MapOrString,),
                                (MapOrString,),
                            >(&mut store, &self.variant_roundtrip)?
                            .func();
                        let all_key_types = *_instance
                            .get_typed_func::<
                                (
                                    &wasmtime::component::__internal::BTreeMap<bool, u32>,
                                    &wasmtime::component::__internal::BTreeMap<u8, u32>,
                                    &wasmtime::component::__internal::BTreeMap<i8, u32>,
                                    &wasmtime::component::__internal::BTreeMap<u16, u32>,
                                    &wasmtime::component::__internal::BTreeMap<i16, u32>,
                                    &wasmtime::component::__internal::BTreeMap<u32, u32>,
                                    &wasmtime::component::__internal::BTreeMap<i32, u32>,
                                    &wasmtime::component::__internal::BTreeMap<u64, u32>,
                                    &wasmtime::component::__internal::BTreeMap<i64, u32>,
                                    &wasmtime::component::__internal::BTreeMap<char, u32>,
                                    &wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        u32,
                                    >,
                                ),
                                (),
                            >(&mut store, &self.all_key_types)?
                            .func();
                        let own_values = *_instance
                            .get_typed_func::<
                                (
                                    &wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::ResourceAny,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::ResourceAny,
                                    >,
                                ),
                            >(&mut store, &self.own_values)?
                            .func();
                        let borrowed_values = *_instance
                            .get_typed_func::<
                                (
                                    &wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::ResourceAny,
                                    >,
                                ),
                                (),
                            >(&mut store, &self.borrowed_values)?
                            .func();
                        Ok(Guest {
                            named_roundtrip,
                            bytes_roundtrip,
                            option_roundtrip,
                            result_roundtrip,
                            tuple_roundtrip,
                            nested_roundtrip,
                            list_of_maps,
                            record_roundtrip,
                            variant_roundtrip,
                            all_key_types,
                            own_values,
                            borrowed_values,
                        })
                    }
                }
                impl Guest {
                    pub fn func_named_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<(NamesById,), (IdsByName,)> {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (NamesById,),
                                (IdsByName,),
                            >::new_unchecked(self.named_roundtrip)
                        }
                    }
                    pub async fn call_named_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: NamesById,
                    ) -> wasmtime::Result<IdsByName>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_named_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_bytes_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::Vec<u8>,
                            >,
                        ),
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::Vec<u8>,
                            >,
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::Vec<u8>,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::Vec<u8>,
                                    >,
                                ),
                            >::new_unchecked(self.bytes_roundtrip)
                        }
                    }
                    pub async fn call_bytes_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::Vec<u8>,
                        >,
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::Vec<u8>,
                        >,
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_bytes_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_option_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            Option<
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                            >,
                        ),
                        (
                            Option<
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                            >,
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    Option<
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                                (
                                    Option<
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                            >::new_unchecked(self.option_roundtrip)
                        }
                    }
                    pub async fn call_option_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: Option<
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                        >,
                    ) -> wasmtime::Result<
                        Option<
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                        >,
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_option_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_result_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            Result<
                                wasmtime::component::__internal::BTreeMap<
                                    u32,
                                    wasmtime::component::__internal::String,
                                >,
                                wasmtime::component::__internal::String,
                            >,
                        ),
                        (
                            Result<
                                wasmtime::component::__internal::BTreeMap<
                                    u32,
                                    wasmtime::component::__internal::String,
                                >,
                                wasmtime::component::__internal::String,
                            >,
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    Result<
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                                (
                                    Result<
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >::new_unchecked(self.result_roundtrip)
                        }
                    }
                    pub async fn call_result_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: Result<
                            wasmtime::component::__internal::BTreeMap<
                                u32,
                                wasmtime::component::__internal::String,
                            >,
                            wasmtime::component::__internal::String,
                        >,
                    ) -> wasmtime::Result<
                        Result<
                            wasmtime::component::__internal::BTreeMap<
                                u32,
                                wasmtime::component::__internal::String,
                            >,
                            wasmtime::component::__internal::String,
                        >,
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_result_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_tuple_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            (
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                                u64,
                            ),
                        ),
                        (
                            (
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                                u64,
                            ),
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    (
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                        u64,
                                    ),
                                ),
                                (
                                    (
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                        u64,
                                    ),
                                ),
                            >::new_unchecked(self.tuple_roundtrip)
                        }
                    }
                    pub async fn call_tuple_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                            u64,
                        ),
                    ) -> wasmtime::Result<
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                            u64,
                        ),
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_tuple_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_nested_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::BTreeMap<
                                    u32,
                                    wasmtime::component::__internal::String,
                                >,
                            >,
                        ),
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::BTreeMap<
                                    u32,
                                    wasmtime::component::__internal::String,
                                >,
                            >,
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::__internal::BTreeMap<
                                            u32,
                                            wasmtime::component::__internal::String,
                                        >,
                                    >,
                                ),
                            >::new_unchecked(self.nested_roundtrip)
                        }
                    }
                    pub async fn call_nested_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::BTreeMap<
                                u32,
                                wasmtime::component::__internal::String,
                            >,
                        >,
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::BTreeMap<
                                u32,
                                wasmtime::component::__internal::String,
                            >,
                        >,
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_nested_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_list_of_maps(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                            >,
                        ),
                        (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::BTreeMap<
                                    wasmtime::component::__internal::String,
                                    u32,
                                >,
                            >,
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::BTreeMap<
                                            wasmtime::component::__internal::String,
                                            u32,
                                        >,
                                    >,
                                ),
                            >::new_unchecked(self.list_of_maps)
                        }
                    }
                    pub async fn call_list_of_maps<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                        >,
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                        >,
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_list_of_maps();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_record_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (LabeledEntry,),
                        (LabeledEntry,),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (LabeledEntry,),
                                (LabeledEntry,),
                            >::new_unchecked(self.record_roundtrip)
                        }
                    }
                    pub async fn call_record_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: LabeledEntry,
                    ) -> wasmtime::Result<LabeledEntry>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_record_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_variant_roundtrip(
                        &self,
                    ) -> wasmtime::component::TypedFunc<(MapOrString,), (MapOrString,)> {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (MapOrString,),
                                (MapOrString,),
                            >::new_unchecked(self.variant_roundtrip)
                        }
                    }
                    pub async fn call_variant_roundtrip<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: MapOrString,
                    ) -> wasmtime::Result<MapOrString>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_variant_roundtrip();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_all_key_types(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            wasmtime::component::__internal::BTreeMap<bool, u32>,
                            wasmtime::component::__internal::BTreeMap<u8, u32>,
                            wasmtime::component::__internal::BTreeMap<i8, u32>,
                            wasmtime::component::__internal::BTreeMap<u16, u32>,
                            wasmtime::component::__internal::BTreeMap<i16, u32>,
                            wasmtime::component::__internal::BTreeMap<u32, u32>,
                            wasmtime::component::__internal::BTreeMap<i32, u32>,
                            wasmtime::component::__internal::BTreeMap<u64, u32>,
                            wasmtime::component::__internal::BTreeMap<i64, u32>,
                            wasmtime::component::__internal::BTreeMap<char, u32>,
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                u32,
                            >,
                        ),
                        (),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::BTreeMap<bool, u32>,
                                    wasmtime::component::__internal::BTreeMap<u8, u32>,
                                    wasmtime::component::__internal::BTreeMap<i8, u32>,
                                    wasmtime::component::__internal::BTreeMap<u16, u32>,
                                    wasmtime::component::__internal::BTreeMap<i16, u32>,
                                    wasmtime::component::__internal::BTreeMap<u32, u32>,
                                    wasmtime::component::__internal::BTreeMap<i32, u32>,
                                    wasmtime::component::__internal::BTreeMap<u64, u32>,
                                    wasmtime::component::__internal::BTreeMap<i64, u32>,
                                    wasmtime::component::__internal::BTreeMap<char, u32>,
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        u32,
                                    >,
                                ),
                                (),
                            >::new_unchecked(self.all_key_types)
                        }
                    }
                    pub async fn call_all_key_types<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: wasmtime::component::__internal::BTreeMap<bool, u32>,
                        arg1: wasmtime::component::__internal::BTreeMap<u8, u32>,
                        arg2: wasmtime::component::__internal::BTreeMap<i8, u32>,
                        arg3: wasmtime::component::__internal::BTreeMap<u16, u32>,
                        arg4: wasmtime::component::__internal::BTreeMap<i16, u32>,
                        arg5: wasmtime::component::__internal::BTreeMap<u32, u32>,
                        arg6: wasmtime::component::__internal::BTreeMap<i32, u32>,
                        arg7: wasmtime::component::__internal::BTreeMap<u64, u32>,
                        arg8: wasmtime::component::__internal::BTreeMap<i64, u32>,
                        arg9: wasmtime::component::__internal::BTreeMap<char, u32>,
                        arg10: wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            u32,
                        >,
                    ) -> wasmtime::Result<()>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_all_key_types();
                        let () = callee
                            .call_concurrent(
                                accessor,
                                (
                                    arg0,
                                    arg1,
                                    arg2,
                                    arg3,
                                    arg4,
                                    arg5,
                                    arg6,
                                    arg7,
                                    arg8,
                                    arg9,
                                    arg10,
                                ),
                            )
                            .await?;
                        Ok(())
                    }
                    pub fn func_own_values(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::ResourceAny,
                            >,
                        ),
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::ResourceAny,
                            >,
                        ),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::ResourceAny,
                                    >,
                                ),
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::ResourceAny,
                                    >,
                                ),
                            >::new_unchecked(self.own_values)
                        }
                    }
                    pub async fn call_own_values<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::ResourceAny,
                        >,
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::ResourceAny,
                        >,
                    >
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_own_values();
                        let (ret0,) = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(ret0)
                    }
                    pub fn func_borrowed_values(
                        &self,
                    ) -> wasmtime::component::TypedFunc<
                        (
                            wasmtime::component::__internal::BTreeMap<
                                wasmtime::component::__internal::String,
                                wasmtime::component::ResourceAny,
                            >,
                        ),
                        (),
                    > {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::BTreeMap<
                                        wasmtime::component::__internal::String,
                                        wasmtime::component::ResourceAny,
                                    >,
                                ),
                                (),
                            >::new_unchecked(self.borrowed_values)
                        }
                    }
                    pub async fn call_borrowed_values<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                        arg0: wasmtime::component::__internal::BTreeMap<
                            wasmtime::component::__internal::String,
                            wasmtime::component::ResourceAny,
                        >,
                    ) -> wasmtime::Result<()>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_borrowed_values();
                        let () = callee.call_concurrent(accessor, (arg0,)).await?;
                        Ok(())
                    }
                }
            }
        }
    }
}
