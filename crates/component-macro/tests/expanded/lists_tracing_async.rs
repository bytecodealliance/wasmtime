/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-lists`.
///
/// This structure is created through [`TheListsPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`TheLists`] as well.
pub struct TheListsPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: TheListsIndices,
}
impl<T> Clone for TheListsPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> TheListsPre<_T> {
    /// Creates a new copy of `TheListsPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = TheListsIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`TheLists`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<TheLists>
    where
        _T: Send,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `the-lists`.
///
/// This is an implementation detail of [`TheListsPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`TheLists`] as well.
#[derive(Clone)]
pub struct TheListsIndices {
    interface0: exports::foo::foo::lists::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-lists`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`TheLists::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`TheListsPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`TheListsPre::instantiate_async`] to
///   create a [`TheLists`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`TheLists::new`].
///
/// * You can also access the guts of instantiation through
///   [`TheListsIndices::new_instance`] followed
///   by [`TheListsIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct TheLists {
    interface0: exports::foo::foo::lists::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheListsIndices {
        /// Creates a new copy of `TheListsIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            let interface0 = exports::foo::foo::lists::GuestIndices::new(_component)?;
            Ok(TheListsIndices { interface0 })
        }
        /// Creates a new instance of [`TheListsIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`TheLists`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`TheLists`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            let interface0 = exports::foo::foo::lists::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            Ok(TheListsIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`TheLists`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheLists> {
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheLists { interface0 })
        }
    }
    impl TheLists {
        /// Convenience wrapper around [`TheListsPre::new`] and
        /// [`TheListsPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheLists>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            TheListsPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`TheListsIndices::new_instance`] and
        /// [`TheListsIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheLists> {
            let indices = TheListsIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::lists::Host + Send,
        {
            foo::foo::lists::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_lists(&self) -> &exports::foo::foo::lists::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod lists {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct OtherRecord {
                #[component(name = "a1")]
                pub a1: u32,
                #[component(name = "a2")]
                pub a2: u64,
                #[component(name = "a3")]
                pub a3: i32,
                #[component(name = "a4")]
                pub a4: i64,
                #[component(name = "b")]
                pub b: wasmtime::component::__internal::String,
                #[component(name = "c")]
                pub c: wasmtime::component::__internal::Vec<u8>,
            }
            impl core::fmt::Debug for OtherRecord {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("OtherRecord")
                        .field("a1", &self.a1)
                        .field("a2", &self.a2)
                        .field("a3", &self.a3)
                        .field("a4", &self.a4)
                        .field("b", &self.b)
                        .field("c", &self.c)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    48 == < OtherRecord as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    8 == < OtherRecord as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct SomeRecord {
                #[component(name = "x")]
                pub x: wasmtime::component::__internal::String,
                #[component(name = "y")]
                pub y: OtherRecord,
                #[component(name = "z")]
                pub z: wasmtime::component::__internal::Vec<OtherRecord>,
                #[component(name = "c1")]
                pub c1: u32,
                #[component(name = "c2")]
                pub c2: u64,
                #[component(name = "c3")]
                pub c3: i32,
                #[component(name = "c4")]
                pub c4: i64,
            }
            impl core::fmt::Debug for SomeRecord {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("SomeRecord")
                        .field("x", &self.x)
                        .field("y", &self.y)
                        .field("z", &self.z)
                        .field("c1", &self.c1)
                        .field("c2", &self.c2)
                        .field("c3", &self.c3)
                        .field("c4", &self.c4)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    96 == < SomeRecord as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    8 == < SomeRecord as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum OtherVariant {
                #[component(name = "a")]
                A,
                #[component(name = "b")]
                B(u32),
                #[component(name = "c")]
                C(wasmtime::component::__internal::String),
            }
            impl core::fmt::Debug for OtherVariant {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        OtherVariant::A => f.debug_tuple("OtherVariant::A").finish(),
                        OtherVariant::B(e) => {
                            f.debug_tuple("OtherVariant::B").field(e).finish()
                        }
                        OtherVariant::C(e) => {
                            f.debug_tuple("OtherVariant::C").field(e).finish()
                        }
                    }
                }
            }
            const _: () = {
                assert!(
                    12 == < OtherVariant as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < OtherVariant as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum SomeVariant {
                #[component(name = "a")]
                A(wasmtime::component::__internal::String),
                #[component(name = "b")]
                B,
                #[component(name = "c")]
                C(u32),
                #[component(name = "d")]
                D(wasmtime::component::__internal::Vec<OtherVariant>),
            }
            impl core::fmt::Debug for SomeVariant {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        SomeVariant::A(e) => {
                            f.debug_tuple("SomeVariant::A").field(e).finish()
                        }
                        SomeVariant::B => f.debug_tuple("SomeVariant::B").finish(),
                        SomeVariant::C(e) => {
                            f.debug_tuple("SomeVariant::C").field(e).finish()
                        }
                        SomeVariant::D(e) => {
                            f.debug_tuple("SomeVariant::D").field(e).finish()
                        }
                    }
                }
            }
            const _: () = {
                assert!(
                    12 == < SomeVariant as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < SomeVariant as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub type LoadStoreAllSizes = wasmtime::component::__internal::Vec<
                (
                    wasmtime::component::__internal::String,
                    u8,
                    i8,
                    u16,
                    i16,
                    u32,
                    i32,
                    u64,
                    i64,
                    f32,
                    f64,
                    char,
                ),
            >;
            const _: () = {
                assert!(
                    8 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                    >::SIZE32
                );
                assert!(
                    4 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                    >::ALIGN32
                );
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn list_u8_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u8>,
                ) -> ();
                async fn list_u16_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u16>,
                ) -> ();
                async fn list_u32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u32>,
                ) -> ();
                async fn list_u64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u64>,
                ) -> ();
                async fn list_s8_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i8>,
                ) -> ();
                async fn list_s16_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i16>,
                ) -> ();
                async fn list_s32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i32>,
                ) -> ();
                async fn list_s64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i64>,
                ) -> ();
                async fn list_float32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<f32>,
                ) -> ();
                async fn list_float64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<f64>,
                ) -> ();
                async fn list_u8_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u8>;
                async fn list_u16_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u16>;
                async fn list_u32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u32>;
                async fn list_u64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u64>;
                async fn list_s8_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i8>;
                async fn list_s16_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i16>;
                async fn list_s32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i32>;
                async fn list_s64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i64>;
                async fn list_float32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<f32>;
                async fn list_float64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<f64>;
                async fn tuple_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<(u8, i8)>,
                ) -> wasmtime::component::__internal::Vec<(i64, u32)>;
                async fn string_list_arg(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::String,
                    >,
                ) -> ();
                async fn string_list_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::String,
                >;
                async fn tuple_string_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        (u8, wasmtime::component::__internal::String),
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    (wasmtime::component::__internal::String, u8),
                >;
                async fn string_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::String,
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::String,
                >;
                async fn record_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<SomeRecord>,
                ) -> wasmtime::component::__internal::Vec<OtherRecord>;
                async fn record_list_reverse(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<OtherRecord>,
                ) -> wasmtime::component::__internal::Vec<SomeRecord>;
                async fn variant_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<SomeVariant>,
                ) -> wasmtime::component::__internal::Vec<OtherVariant>;
                async fn load_store_everything(
                    &mut self,
                    a: LoadStoreAllSizes,
                ) -> LoadStoreAllSizes;
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
                let mut inst = linker.instance("foo:foo/lists")?;
                inst.func_wrap_async(
                    "list-u8-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u8>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u8-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u8_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-u16-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u16>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u16-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u16_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-u32-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u32>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u32-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u32_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-u64-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u64>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u64-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u64_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-s8-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i8>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s8-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s8_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-s16-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i16>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s16-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s16_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-s32-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i32>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s32-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s32_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-s64-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i64>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s64-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s64_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-float32-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<f32>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-float32-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_float32_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-float64-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<f64>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-float64-param",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_float64_param(host, arg0).await;
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
                inst.func_wrap_async(
                    "list-u8-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u8-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u8_ret(host).await;
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
                    "list-u16-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u16-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u16_ret(host).await;
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
                    "list-u32-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u32-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u32_ret(host).await;
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
                    "list-u64-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-u64-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_u64_ret(host).await;
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
                    "list-s8-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s8-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s8_ret(host).await;
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
                    "list-s16-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s16-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s16_ret(host).await;
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
                    "list-s32-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s32-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s32_ret(host).await;
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
                    "list-s64-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-s64-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_s64_ret(host).await;
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
                    "list-float32-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-float32-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_float32_ret(host).await;
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
                    "list-float64-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "list-float64-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_float64_ret(host).await;
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
                    "tuple-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<(u8, i8)>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "tuple-list",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::tuple_list(host, arg0).await;
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
                    "string-list-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::String,
                            >,
                        )|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "string-list-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, a = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::string_list_arg(host, arg0).await;
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
                inst.func_wrap_async(
                    "string-list-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "string-list-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::string_list_ret(host).await;
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
                    "tuple-string-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                (u8, wasmtime::component::__internal::String),
                            >,
                        )|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "tuple-string-list",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::tuple_string_list(host, arg0).await;
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
                    "string-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::String,
                            >,
                        )|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "string-list",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::string_list(host, arg0).await;
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
                    "record-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<SomeRecord>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "record-list",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::record_list(host, arg0).await;
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
                    "record-list-reverse",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<OtherRecord>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "record-list-reverse",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::record_list_reverse(host, arg0).await;
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
                    "variant-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<SomeVariant>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "variant-list",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::variant_list(host, arg0).await;
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
                    "load-store-everything",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (LoadStoreAllSizes,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "lists", function = "load-store-everything",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, a = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::load_store_everything(host, arg0).await;
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
                async fn list_u8_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u8>,
                ) -> () {
                    Host::list_u8_param(*self, x).await
                }
                async fn list_u16_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u16>,
                ) -> () {
                    Host::list_u16_param(*self, x).await
                }
                async fn list_u32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u32>,
                ) -> () {
                    Host::list_u32_param(*self, x).await
                }
                async fn list_u64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u64>,
                ) -> () {
                    Host::list_u64_param(*self, x).await
                }
                async fn list_s8_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i8>,
                ) -> () {
                    Host::list_s8_param(*self, x).await
                }
                async fn list_s16_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i16>,
                ) -> () {
                    Host::list_s16_param(*self, x).await
                }
                async fn list_s32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i32>,
                ) -> () {
                    Host::list_s32_param(*self, x).await
                }
                async fn list_s64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i64>,
                ) -> () {
                    Host::list_s64_param(*self, x).await
                }
                async fn list_float32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<f32>,
                ) -> () {
                    Host::list_float32_param(*self, x).await
                }
                async fn list_float64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<f64>,
                ) -> () {
                    Host::list_float64_param(*self, x).await
                }
                async fn list_u8_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u8> {
                    Host::list_u8_ret(*self).await
                }
                async fn list_u16_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u16> {
                    Host::list_u16_ret(*self).await
                }
                async fn list_u32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u32> {
                    Host::list_u32_ret(*self).await
                }
                async fn list_u64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u64> {
                    Host::list_u64_ret(*self).await
                }
                async fn list_s8_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i8> {
                    Host::list_s8_ret(*self).await
                }
                async fn list_s16_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i16> {
                    Host::list_s16_ret(*self).await
                }
                async fn list_s32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i32> {
                    Host::list_s32_ret(*self).await
                }
                async fn list_s64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i64> {
                    Host::list_s64_ret(*self).await
                }
                async fn list_float32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<f32> {
                    Host::list_float32_ret(*self).await
                }
                async fn list_float64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<f64> {
                    Host::list_float64_ret(*self).await
                }
                async fn tuple_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<(u8, i8)>,
                ) -> wasmtime::component::__internal::Vec<(i64, u32)> {
                    Host::tuple_list(*self, x).await
                }
                async fn string_list_arg(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::String,
                    >,
                ) -> () {
                    Host::string_list_arg(*self, a).await
                }
                async fn string_list_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::String,
                > {
                    Host::string_list_ret(*self).await
                }
                async fn tuple_string_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        (u8, wasmtime::component::__internal::String),
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    (wasmtime::component::__internal::String, u8),
                > {
                    Host::tuple_string_list(*self, x).await
                }
                async fn string_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::String,
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::String,
                > {
                    Host::string_list(*self, x).await
                }
                async fn record_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<SomeRecord>,
                ) -> wasmtime::component::__internal::Vec<OtherRecord> {
                    Host::record_list(*self, x).await
                }
                async fn record_list_reverse(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<OtherRecord>,
                ) -> wasmtime::component::__internal::Vec<SomeRecord> {
                    Host::record_list_reverse(*self, x).await
                }
                async fn variant_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<SomeVariant>,
                ) -> wasmtime::component::__internal::Vec<OtherVariant> {
                    Host::variant_list(*self, x).await
                }
                async fn load_store_everything(
                    &mut self,
                    a: LoadStoreAllSizes,
                ) -> LoadStoreAllSizes {
                    Host::load_store_everything(*self, a).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod lists {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct OtherRecord {
                    #[component(name = "a1")]
                    pub a1: u32,
                    #[component(name = "a2")]
                    pub a2: u64,
                    #[component(name = "a3")]
                    pub a3: i32,
                    #[component(name = "a4")]
                    pub a4: i64,
                    #[component(name = "b")]
                    pub b: wasmtime::component::__internal::String,
                    #[component(name = "c")]
                    pub c: wasmtime::component::__internal::Vec<u8>,
                }
                impl core::fmt::Debug for OtherRecord {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("OtherRecord")
                            .field("a1", &self.a1)
                            .field("a2", &self.a2)
                            .field("a3", &self.a3)
                            .field("a4", &self.a4)
                            .field("b", &self.b)
                            .field("c", &self.c)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        48 == < OtherRecord as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        8 == < OtherRecord as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct SomeRecord {
                    #[component(name = "x")]
                    pub x: wasmtime::component::__internal::String,
                    #[component(name = "y")]
                    pub y: OtherRecord,
                    #[component(name = "z")]
                    pub z: wasmtime::component::__internal::Vec<OtherRecord>,
                    #[component(name = "c1")]
                    pub c1: u32,
                    #[component(name = "c2")]
                    pub c2: u64,
                    #[component(name = "c3")]
                    pub c3: i32,
                    #[component(name = "c4")]
                    pub c4: i64,
                }
                impl core::fmt::Debug for SomeRecord {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("SomeRecord")
                            .field("x", &self.x)
                            .field("y", &self.y)
                            .field("z", &self.z)
                            .field("c1", &self.c1)
                            .field("c2", &self.c2)
                            .field("c3", &self.c3)
                            .field("c4", &self.c4)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        96 == < SomeRecord as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        8 == < SomeRecord as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone)]
                pub enum OtherVariant {
                    #[component(name = "a")]
                    A,
                    #[component(name = "b")]
                    B(u32),
                    #[component(name = "c")]
                    C(wasmtime::component::__internal::String),
                }
                impl core::fmt::Debug for OtherVariant {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            OtherVariant::A => f.debug_tuple("OtherVariant::A").finish(),
                            OtherVariant::B(e) => {
                                f.debug_tuple("OtherVariant::B").field(e).finish()
                            }
                            OtherVariant::C(e) => {
                                f.debug_tuple("OtherVariant::C").field(e).finish()
                            }
                        }
                    }
                }
                const _: () = {
                    assert!(
                        12 == < OtherVariant as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < OtherVariant as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone)]
                pub enum SomeVariant {
                    #[component(name = "a")]
                    A(wasmtime::component::__internal::String),
                    #[component(name = "b")]
                    B,
                    #[component(name = "c")]
                    C(u32),
                    #[component(name = "d")]
                    D(wasmtime::component::__internal::Vec<OtherVariant>),
                }
                impl core::fmt::Debug for SomeVariant {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            SomeVariant::A(e) => {
                                f.debug_tuple("SomeVariant::A").field(e).finish()
                            }
                            SomeVariant::B => f.debug_tuple("SomeVariant::B").finish(),
                            SomeVariant::C(e) => {
                                f.debug_tuple("SomeVariant::C").field(e).finish()
                            }
                            SomeVariant::D(e) => {
                                f.debug_tuple("SomeVariant::D").field(e).finish()
                            }
                        }
                    }
                }
                const _: () = {
                    assert!(
                        12 == < SomeVariant as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < SomeVariant as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub type LoadStoreAllSizes = wasmtime::component::__internal::Vec<
                    (
                        wasmtime::component::__internal::String,
                        u8,
                        i8,
                        u16,
                        i16,
                        u32,
                        i32,
                        u64,
                        i64,
                        f32,
                        f64,
                        char,
                    ),
                >;
                const _: () = {
                    assert!(
                        8 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub struct Guest {
                    list_u8_param: wasmtime::component::Func,
                    list_u16_param: wasmtime::component::Func,
                    list_u32_param: wasmtime::component::Func,
                    list_u64_param: wasmtime::component::Func,
                    list_s8_param: wasmtime::component::Func,
                    list_s16_param: wasmtime::component::Func,
                    list_s32_param: wasmtime::component::Func,
                    list_s64_param: wasmtime::component::Func,
                    list_float32_param: wasmtime::component::Func,
                    list_float64_param: wasmtime::component::Func,
                    list_u8_ret: wasmtime::component::Func,
                    list_u16_ret: wasmtime::component::Func,
                    list_u32_ret: wasmtime::component::Func,
                    list_u64_ret: wasmtime::component::Func,
                    list_s8_ret: wasmtime::component::Func,
                    list_s16_ret: wasmtime::component::Func,
                    list_s32_ret: wasmtime::component::Func,
                    list_s64_ret: wasmtime::component::Func,
                    list_float32_ret: wasmtime::component::Func,
                    list_float64_ret: wasmtime::component::Func,
                    tuple_list: wasmtime::component::Func,
                    string_list_arg: wasmtime::component::Func,
                    string_list_ret: wasmtime::component::Func,
                    tuple_string_list: wasmtime::component::Func,
                    string_list: wasmtime::component::Func,
                    record_list: wasmtime::component::Func,
                    record_list_reverse: wasmtime::component::Func,
                    variant_list: wasmtime::component::Func,
                    load_store_everything: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    list_u8_param: wasmtime::component::ComponentExportIndex,
                    list_u16_param: wasmtime::component::ComponentExportIndex,
                    list_u32_param: wasmtime::component::ComponentExportIndex,
                    list_u64_param: wasmtime::component::ComponentExportIndex,
                    list_s8_param: wasmtime::component::ComponentExportIndex,
                    list_s16_param: wasmtime::component::ComponentExportIndex,
                    list_s32_param: wasmtime::component::ComponentExportIndex,
                    list_s64_param: wasmtime::component::ComponentExportIndex,
                    list_float32_param: wasmtime::component::ComponentExportIndex,
                    list_float64_param: wasmtime::component::ComponentExportIndex,
                    list_u8_ret: wasmtime::component::ComponentExportIndex,
                    list_u16_ret: wasmtime::component::ComponentExportIndex,
                    list_u32_ret: wasmtime::component::ComponentExportIndex,
                    list_u64_ret: wasmtime::component::ComponentExportIndex,
                    list_s8_ret: wasmtime::component::ComponentExportIndex,
                    list_s16_ret: wasmtime::component::ComponentExportIndex,
                    list_s32_ret: wasmtime::component::ComponentExportIndex,
                    list_s64_ret: wasmtime::component::ComponentExportIndex,
                    list_float32_ret: wasmtime::component::ComponentExportIndex,
                    list_float64_ret: wasmtime::component::ComponentExportIndex,
                    tuple_list: wasmtime::component::ComponentExportIndex,
                    string_list_arg: wasmtime::component::ComponentExportIndex,
                    string_list_ret: wasmtime::component::ComponentExportIndex,
                    tuple_string_list: wasmtime::component::ComponentExportIndex,
                    string_list: wasmtime::component::ComponentExportIndex,
                    record_list: wasmtime::component::ComponentExportIndex,
                    record_list_reverse: wasmtime::component::ComponentExportIndex,
                    variant_list: wasmtime::component::ComponentExportIndex,
                    load_store_everything: wasmtime::component::ComponentExportIndex,
                }
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestIndices> {
                        let (_, instance) = component
                            .export_index(None, "foo:foo/lists")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/lists`"
                                )
                            })?;
                        Self::_new(|name| {
                            component.export_index(Some(&instance), name).map(|p| p.1)
                        })
                    }
                    /// This constructor is similar to [`GuestIndices::new`] except that it
                    /// performs string lookups after instantiation time.
                    pub fn new_instance(
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance_export = instance
                            .get_export(&mut store, None, "foo:foo/lists")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/lists`"
                                )
                            })?;
                        Self::_new(|name| {
                            instance.get_export(&mut store, Some(&instance_export), name)
                        })
                    }
                    fn _new(
                        mut lookup: impl FnMut(
                            &str,
                        ) -> Option<wasmtime::component::ComponentExportIndex>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let mut lookup = move |name| {
                            lookup(name)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/lists` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let list_u8_param = lookup("list-u8-param")?;
                        let list_u16_param = lookup("list-u16-param")?;
                        let list_u32_param = lookup("list-u32-param")?;
                        let list_u64_param = lookup("list-u64-param")?;
                        let list_s8_param = lookup("list-s8-param")?;
                        let list_s16_param = lookup("list-s16-param")?;
                        let list_s32_param = lookup("list-s32-param")?;
                        let list_s64_param = lookup("list-s64-param")?;
                        let list_float32_param = lookup("list-float32-param")?;
                        let list_float64_param = lookup("list-float64-param")?;
                        let list_u8_ret = lookup("list-u8-ret")?;
                        let list_u16_ret = lookup("list-u16-ret")?;
                        let list_u32_ret = lookup("list-u32-ret")?;
                        let list_u64_ret = lookup("list-u64-ret")?;
                        let list_s8_ret = lookup("list-s8-ret")?;
                        let list_s16_ret = lookup("list-s16-ret")?;
                        let list_s32_ret = lookup("list-s32-ret")?;
                        let list_s64_ret = lookup("list-s64-ret")?;
                        let list_float32_ret = lookup("list-float32-ret")?;
                        let list_float64_ret = lookup("list-float64-ret")?;
                        let tuple_list = lookup("tuple-list")?;
                        let string_list_arg = lookup("string-list-arg")?;
                        let string_list_ret = lookup("string-list-ret")?;
                        let tuple_string_list = lookup("tuple-string-list")?;
                        let string_list = lookup("string-list")?;
                        let record_list = lookup("record-list")?;
                        let record_list_reverse = lookup("record-list-reverse")?;
                        let variant_list = lookup("variant-list")?;
                        let load_store_everything = lookup("load-store-everything")?;
                        Ok(GuestIndices {
                            list_u8_param,
                            list_u16_param,
                            list_u32_param,
                            list_u64_param,
                            list_s8_param,
                            list_s16_param,
                            list_s32_param,
                            list_s64_param,
                            list_float32_param,
                            list_float64_param,
                            list_u8_ret,
                            list_u16_ret,
                            list_u32_ret,
                            list_u64_ret,
                            list_s8_ret,
                            list_s16_ret,
                            list_s32_ret,
                            list_s64_ret,
                            list_float32_ret,
                            list_float64_ret,
                            tuple_list,
                            string_list_arg,
                            string_list_ret,
                            tuple_string_list,
                            string_list,
                            record_list,
                            record_list_reverse,
                            variant_list,
                            load_store_everything,
                        })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let list_u8_param = *_instance
                            .get_typed_func::<
                                (&[u8],),
                                (),
                            >(&mut store, &self.list_u8_param)?
                            .func();
                        let list_u16_param = *_instance
                            .get_typed_func::<
                                (&[u16],),
                                (),
                            >(&mut store, &self.list_u16_param)?
                            .func();
                        let list_u32_param = *_instance
                            .get_typed_func::<
                                (&[u32],),
                                (),
                            >(&mut store, &self.list_u32_param)?
                            .func();
                        let list_u64_param = *_instance
                            .get_typed_func::<
                                (&[u64],),
                                (),
                            >(&mut store, &self.list_u64_param)?
                            .func();
                        let list_s8_param = *_instance
                            .get_typed_func::<
                                (&[i8],),
                                (),
                            >(&mut store, &self.list_s8_param)?
                            .func();
                        let list_s16_param = *_instance
                            .get_typed_func::<
                                (&[i16],),
                                (),
                            >(&mut store, &self.list_s16_param)?
                            .func();
                        let list_s32_param = *_instance
                            .get_typed_func::<
                                (&[i32],),
                                (),
                            >(&mut store, &self.list_s32_param)?
                            .func();
                        let list_s64_param = *_instance
                            .get_typed_func::<
                                (&[i64],),
                                (),
                            >(&mut store, &self.list_s64_param)?
                            .func();
                        let list_float32_param = *_instance
                            .get_typed_func::<
                                (&[f32],),
                                (),
                            >(&mut store, &self.list_float32_param)?
                            .func();
                        let list_float64_param = *_instance
                            .get_typed_func::<
                                (&[f64],),
                                (),
                            >(&mut store, &self.list_float64_param)?
                            .func();
                        let list_u8_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u8>,),
                            >(&mut store, &self.list_u8_ret)?
                            .func();
                        let list_u16_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u16>,),
                            >(&mut store, &self.list_u16_ret)?
                            .func();
                        let list_u32_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >(&mut store, &self.list_u32_ret)?
                            .func();
                        let list_u64_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u64>,),
                            >(&mut store, &self.list_u64_ret)?
                            .func();
                        let list_s8_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i8>,),
                            >(&mut store, &self.list_s8_ret)?
                            .func();
                        let list_s16_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i16>,),
                            >(&mut store, &self.list_s16_ret)?
                            .func();
                        let list_s32_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i32>,),
                            >(&mut store, &self.list_s32_ret)?
                            .func();
                        let list_s64_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i64>,),
                            >(&mut store, &self.list_s64_ret)?
                            .func();
                        let list_float32_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<f32>,),
                            >(&mut store, &self.list_float32_ret)?
                            .func();
                        let list_float64_ret = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<f64>,),
                            >(&mut store, &self.list_float64_ret)?
                            .func();
                        let tuple_list = *_instance
                            .get_typed_func::<
                                (&[(u8, i8)],),
                                (wasmtime::component::__internal::Vec<(i64, u32)>,),
                            >(&mut store, &self.tuple_list)?
                            .func();
                        let string_list_arg = *_instance
                            .get_typed_func::<
                                (&[wasmtime::component::__internal::String],),
                                (),
                            >(&mut store, &self.string_list_arg)?
                            .func();
                        let string_list_ret = *_instance
                            .get_typed_func::<
                                (),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >(&mut store, &self.string_list_ret)?
                            .func();
                        let tuple_string_list = *_instance
                            .get_typed_func::<
                                (&[(u8, wasmtime::component::__internal::String)],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        (wasmtime::component::__internal::String, u8),
                                    >,
                                ),
                            >(&mut store, &self.tuple_string_list)?
                            .func();
                        let string_list = *_instance
                            .get_typed_func::<
                                (&[wasmtime::component::__internal::String],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >(&mut store, &self.string_list)?
                            .func();
                        let record_list = *_instance
                            .get_typed_func::<
                                (&[SomeRecord],),
                                (wasmtime::component::__internal::Vec<OtherRecord>,),
                            >(&mut store, &self.record_list)?
                            .func();
                        let record_list_reverse = *_instance
                            .get_typed_func::<
                                (&[OtherRecord],),
                                (wasmtime::component::__internal::Vec<SomeRecord>,),
                            >(&mut store, &self.record_list_reverse)?
                            .func();
                        let variant_list = *_instance
                            .get_typed_func::<
                                (&[SomeVariant],),
                                (wasmtime::component::__internal::Vec<OtherVariant>,),
                            >(&mut store, &self.variant_list)?
                            .func();
                        let load_store_everything = *_instance
                            .get_typed_func::<
                                (&LoadStoreAllSizes,),
                                (LoadStoreAllSizes,),
                            >(&mut store, &self.load_store_everything)?
                            .func();
                        Ok(Guest {
                            list_u8_param,
                            list_u16_param,
                            list_u32_param,
                            list_u64_param,
                            list_s8_param,
                            list_s16_param,
                            list_s32_param,
                            list_s64_param,
                            list_float32_param,
                            list_float64_param,
                            list_u8_ret,
                            list_u16_ret,
                            list_u32_ret,
                            list_u64_ret,
                            list_s8_ret,
                            list_s16_ret,
                            list_s32_ret,
                            list_s64_ret,
                            list_float32_ret,
                            list_float64_ret,
                            tuple_list,
                            string_list_arg,
                            string_list_ret,
                            tuple_string_list,
                            string_list,
                            record_list,
                            record_list_reverse,
                            variant_list,
                            load_store_everything,
                        })
                    }
                }
                impl Guest {
                    pub async fn call_list_u8_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u8],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u8-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u8],),
                                (),
                            >::new_unchecked(self.list_u8_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u16_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u16],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u16-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u16],),
                                (),
                            >::new_unchecked(self.list_u16_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u32-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u32],),
                                (),
                            >::new_unchecked(self.list_u32_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u64],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u64-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u64],),
                                (),
                            >::new_unchecked(self.list_u64_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s8_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i8],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s8-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i8],),
                                (),
                            >::new_unchecked(self.list_s8_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s16_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i16],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s16-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i16],),
                                (),
                            >::new_unchecked(self.list_s16_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s32-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i32],),
                                (),
                            >::new_unchecked(self.list_s32_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i64],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s64-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i64],),
                                (),
                            >::new_unchecked(self.list_s64_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_float32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[f32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-float32-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[f32],),
                                (),
                            >::new_unchecked(self.list_float32_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_float64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[f64],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-float64-param",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[f64],),
                                (),
                            >::new_unchecked(self.list_float64_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u8_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u8>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u8-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u8>,),
                            >::new_unchecked(self.list_u8_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_u16_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u16>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u16-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u16>,),
                            >::new_unchecked(self.list_u16_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_u32_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u32-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >::new_unchecked(self.list_u32_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_u64_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u64>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-u64-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u64>,),
                            >::new_unchecked(self.list_u64_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s8_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i8>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s8-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i8>,),
                            >::new_unchecked(self.list_s8_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s16_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i16>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s16-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i16>,),
                            >::new_unchecked(self.list_s16_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s32_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s32-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i32>,),
                            >::new_unchecked(self.list_s32_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s64_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i64>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-s64-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i64>,),
                            >::new_unchecked(self.list_s64_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_float32_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<f32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-float32-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<f32>,),
                            >::new_unchecked(self.list_float32_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_float64_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<f64>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "list-float64-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<f64>,),
                            >::new_unchecked(self.list_float64_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_tuple_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[(u8, i8)],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<(i64, u32)>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "tuple-list",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[(u8, i8)],),
                                (wasmtime::component::__internal::Vec<(i64, u32)>,),
                            >::new_unchecked(self.tuple_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_string_list_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[wasmtime::component::__internal::String],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "string-list-arg",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[wasmtime::component::__internal::String],),
                                (),
                            >::new_unchecked(self.string_list_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_string_list_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::String,
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "string-list-ret",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >::new_unchecked(self.string_list_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_tuple_string_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[(u8, wasmtime::component::__internal::String)],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            (wasmtime::component::__internal::String, u8),
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "tuple-string-list",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[(u8, wasmtime::component::__internal::String)],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        (wasmtime::component::__internal::String, u8),
                                    >,
                                ),
                            >::new_unchecked(self.tuple_string_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_string_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[wasmtime::component::__internal::String],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::String,
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "string-list",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[wasmtime::component::__internal::String],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >::new_unchecked(self.string_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_record_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[SomeRecord],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<OtherRecord>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "record-list",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[SomeRecord],),
                                (wasmtime::component::__internal::Vec<OtherRecord>,),
                            >::new_unchecked(self.record_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_record_list_reverse<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[OtherRecord],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<SomeRecord>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "record-list-reverse",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[OtherRecord],),
                                (wasmtime::component::__internal::Vec<SomeRecord>,),
                            >::new_unchecked(self.record_list_reverse)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_variant_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[SomeVariant],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<OtherVariant>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "variant-list",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[SomeVariant],),
                                (wasmtime::component::__internal::Vec<OtherVariant>,),
                            >::new_unchecked(self.variant_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_load_store_everything<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &LoadStoreAllSizes,
                    ) -> wasmtime::Result<LoadStoreAllSizes>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/lists", function = "load-store-everything",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&LoadStoreAllSizes,),
                                (LoadStoreAllSizes,),
                            >::new_unchecked(self.load_store_everything)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
