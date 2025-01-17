/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`TheWorld`] as well.
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: TheWorldIndices,
}
impl<T> Clone for TheWorldPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> TheWorldPre<_T> {
    /// Creates a new copy of `TheWorldPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = TheWorldIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`TheWorld`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<TheWorld>
    where
        _T: Send + 'static,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `the-world`.
///
/// This is an implementation detail of [`TheWorldPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`TheWorld`] as well.
#[derive(Clone)]
pub struct TheWorldIndices {
    interface0: exports::foo::foo::records::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`TheWorld::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`TheWorldPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`TheWorldPre::instantiate_async`] to
///   create a [`TheWorld`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`TheWorld::new`].
///
/// * You can also access the guts of instantiation through
///   [`TheWorldIndices::new_instance`] followed
///   by [`TheWorldIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct TheWorld {
    interface0: exports::foo::foo::records::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorldIndices {
        /// Creates a new copy of `TheWorldIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            let interface0 = exports::foo::foo::records::GuestIndices::new(_component)?;
            Ok(TheWorldIndices { interface0 })
        }
        /// Creates a new instance of [`TheWorldIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`TheWorld`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`TheWorld`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            let interface0 = exports::foo::foo::records::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            Ok(TheWorldIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`TheWorld`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheWorld { interface0 })
        }
    }
    impl TheWorld {
        /// Convenience wrapper around [`TheWorldPre::new`] and
        /// [`TheWorldPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheWorld>
        where
            _T: Send + 'static,
        {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`TheWorldIndices::new_instance`] and
        /// [`TheWorldIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let indices = TheWorldIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send + foo::foo::records::Host<Data = T> + 'static,
            U: Send + foo::foo::records::Host<Data = T>,
        {
            foo::foo::records::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_records(&self) -> &exports::foo::foo::records::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod records {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct Empty {}
            impl core::fmt::Debug for Empty {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Empty").finish()
                }
            }
            const _: () = {
                assert!(0 == < Empty as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Empty as wasmtime::component::ComponentType >::ALIGN32);
            };
            /// A record containing two scalar fields
            /// that both have the same type
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct Scalars {
                /// The first field, named a
                #[component(name = "a")]
                pub a: u32,
                /// The second field, named b
                #[component(name = "b")]
                pub b: u32,
            }
            impl core::fmt::Debug for Scalars {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Scalars")
                        .field("a", &self.a)
                        .field("b", &self.b)
                        .finish()
                }
            }
            const _: () = {
                assert!(8 == < Scalars as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Scalars as wasmtime::component::ComponentType >::ALIGN32);
            };
            /// A record that is really just flags
            /// All of the fields are bool
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct ReallyFlags {
                #[component(name = "a")]
                pub a: bool,
                #[component(name = "b")]
                pub b: bool,
                #[component(name = "c")]
                pub c: bool,
                #[component(name = "d")]
                pub d: bool,
                #[component(name = "e")]
                pub e: bool,
                #[component(name = "f")]
                pub f: bool,
                #[component(name = "g")]
                pub g: bool,
                #[component(name = "h")]
                pub h: bool,
                #[component(name = "i")]
                pub i: bool,
            }
            impl core::fmt::Debug for ReallyFlags {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("ReallyFlags")
                        .field("a", &self.a)
                        .field("b", &self.b)
                        .field("c", &self.c)
                        .field("d", &self.d)
                        .field("e", &self.e)
                        .field("f", &self.f)
                        .field("g", &self.g)
                        .field("h", &self.h)
                        .field("i", &self.i)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    9 == < ReallyFlags as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    1 == < ReallyFlags as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct Aggregates {
                #[component(name = "a")]
                pub a: Scalars,
                #[component(name = "b")]
                pub b: u32,
                #[component(name = "c")]
                pub c: Empty,
                #[component(name = "d")]
                pub d: wasmtime::component::__internal::String,
                #[component(name = "e")]
                pub e: ReallyFlags,
            }
            impl core::fmt::Debug for Aggregates {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Aggregates")
                        .field("a", &self.a)
                        .field("b", &self.b)
                        .field("c", &self.c)
                        .field("d", &self.d)
                        .field("e", &self.e)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    32 == < Aggregates as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < Aggregates as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub type IntTypedef = i32;
            const _: () = {
                assert!(
                    4 == < IntTypedef as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < IntTypedef as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub type TupleTypedef2 = (IntTypedef,);
            const _: () = {
                assert!(
                    4 == < TupleTypedef2 as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < TupleTypedef2 as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub trait Host {
                type Data;
                fn tuple_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: (char, u32),
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn tuple_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> (char, u32) + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn empty_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: Empty,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn empty_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Empty + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn scalar_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: Scalars,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn scalar_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Scalars + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn flags_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: ReallyFlags,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn flags_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> ReallyFlags + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn aggregate_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: Aggregates,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn aggregate_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Aggregates + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn typedef_inout(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    e: TupleTypedef2,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> i32 + Send + Sync + 'static,
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
                let mut inst = linker.instance("foo:foo/records")?;
                inst.func_wrap_concurrent(
                    "tuple-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): ((char, u32),)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::tuple_arg(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "tuple-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::tuple_result(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<((char, u32),)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<((char, u32),)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "empty-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Empty,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::empty_arg(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "empty-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::empty_result(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(Empty,)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(Empty,)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "scalar-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Scalars,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::scalar_arg(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "scalar-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::scalar_result(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(Scalars,)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(Scalars,)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "flags-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (ReallyFlags,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::flags_arg(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "flags-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::flags_result(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(ReallyFlags,)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(ReallyFlags,)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "aggregate-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Aggregates,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::aggregate_arg(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "aggregate-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::aggregate_result(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(Aggregates,)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(Aggregates,)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "typedef-inout",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (TupleTypedef2,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::typedef_inout(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(i32,)> + Send + Sync,
                                >
                        })
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(i32,)> + Send + Sync,
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
                fn tuple_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: (char, u32),
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::tuple_arg(store, x)
                }
                fn tuple_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> (char, u32) + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::tuple_result(store)
                }
                fn empty_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: Empty,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::empty_arg(store, x)
                }
                fn empty_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Empty + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::empty_result(store)
                }
                fn scalar_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: Scalars,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::scalar_arg(store, x)
                }
                fn scalar_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Scalars + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::scalar_result(store)
                }
                fn flags_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: ReallyFlags,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::flags_arg(store, x)
                }
                fn flags_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> ReallyFlags + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::flags_result(store)
                }
                fn aggregate_arg(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: Aggregates,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::aggregate_arg(store, x)
                }
                fn aggregate_result(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> Aggregates + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::aggregate_result(store)
                }
                fn typedef_inout(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    e: TupleTypedef2,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> i32 + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::typedef_inout(store, e)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod records {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::{anyhow, Box};
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone, Copy)]
                pub struct Empty {}
                impl core::fmt::Debug for Empty {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("Empty").finish()
                    }
                }
                const _: () = {
                    assert!(
                        0 == < Empty as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < Empty as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                /// A record containing two scalar fields
                /// that both have the same type
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone, Copy)]
                pub struct Scalars {
                    /// The first field, named a
                    #[component(name = "a")]
                    pub a: u32,
                    /// The second field, named b
                    #[component(name = "b")]
                    pub b: u32,
                }
                impl core::fmt::Debug for Scalars {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("Scalars")
                            .field("a", &self.a)
                            .field("b", &self.b)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        8 == < Scalars as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < Scalars as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                /// A record that is really just flags
                /// All of the fields are bool
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone, Copy)]
                pub struct ReallyFlags {
                    #[component(name = "a")]
                    pub a: bool,
                    #[component(name = "b")]
                    pub b: bool,
                    #[component(name = "c")]
                    pub c: bool,
                    #[component(name = "d")]
                    pub d: bool,
                    #[component(name = "e")]
                    pub e: bool,
                    #[component(name = "f")]
                    pub f: bool,
                    #[component(name = "g")]
                    pub g: bool,
                    #[component(name = "h")]
                    pub h: bool,
                    #[component(name = "i")]
                    pub i: bool,
                }
                impl core::fmt::Debug for ReallyFlags {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("ReallyFlags")
                            .field("a", &self.a)
                            .field("b", &self.b)
                            .field("c", &self.c)
                            .field("d", &self.d)
                            .field("e", &self.e)
                            .field("f", &self.f)
                            .field("g", &self.g)
                            .field("h", &self.h)
                            .field("i", &self.i)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        9 == < ReallyFlags as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        1 == < ReallyFlags as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct Aggregates {
                    #[component(name = "a")]
                    pub a: Scalars,
                    #[component(name = "b")]
                    pub b: u32,
                    #[component(name = "c")]
                    pub c: Empty,
                    #[component(name = "d")]
                    pub d: wasmtime::component::__internal::String,
                    #[component(name = "e")]
                    pub e: ReallyFlags,
                }
                impl core::fmt::Debug for Aggregates {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("Aggregates")
                            .field("a", &self.a)
                            .field("b", &self.b)
                            .field("c", &self.c)
                            .field("d", &self.d)
                            .field("e", &self.e)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        32 == < Aggregates as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < Aggregates as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub type IntTypedef = i32;
                const _: () = {
                    assert!(
                        4 == < IntTypedef as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < IntTypedef as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub type TupleTypedef2 = (IntTypedef,);
                const _: () = {
                    assert!(
                        4 == < TupleTypedef2 as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < TupleTypedef2 as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub struct Guest {
                    tuple_arg: wasmtime::component::Func,
                    tuple_result: wasmtime::component::Func,
                    empty_arg: wasmtime::component::Func,
                    empty_result: wasmtime::component::Func,
                    scalar_arg: wasmtime::component::Func,
                    scalar_result: wasmtime::component::Func,
                    flags_arg: wasmtime::component::Func,
                    flags_result: wasmtime::component::Func,
                    aggregate_arg: wasmtime::component::Func,
                    aggregate_result: wasmtime::component::Func,
                    typedef_inout: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    tuple_arg: wasmtime::component::ComponentExportIndex,
                    tuple_result: wasmtime::component::ComponentExportIndex,
                    empty_arg: wasmtime::component::ComponentExportIndex,
                    empty_result: wasmtime::component::ComponentExportIndex,
                    scalar_arg: wasmtime::component::ComponentExportIndex,
                    scalar_result: wasmtime::component::ComponentExportIndex,
                    flags_arg: wasmtime::component::ComponentExportIndex,
                    flags_result: wasmtime::component::ComponentExportIndex,
                    aggregate_arg: wasmtime::component::ComponentExportIndex,
                    aggregate_result: wasmtime::component::ComponentExportIndex,
                    typedef_inout: wasmtime::component::ComponentExportIndex,
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
                            .export_index(None, "foo:foo/records")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/records`"
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
                            .get_export(&mut store, None, "foo:foo/records")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/records`"
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
                                        "instance export `foo:foo/records` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let tuple_arg = lookup("tuple-arg")?;
                        let tuple_result = lookup("tuple-result")?;
                        let empty_arg = lookup("empty-arg")?;
                        let empty_result = lookup("empty-result")?;
                        let scalar_arg = lookup("scalar-arg")?;
                        let scalar_result = lookup("scalar-result")?;
                        let flags_arg = lookup("flags-arg")?;
                        let flags_result = lookup("flags-result")?;
                        let aggregate_arg = lookup("aggregate-arg")?;
                        let aggregate_result = lookup("aggregate-result")?;
                        let typedef_inout = lookup("typedef-inout")?;
                        Ok(GuestIndices {
                            tuple_arg,
                            tuple_result,
                            empty_arg,
                            empty_result,
                            scalar_arg,
                            scalar_result,
                            flags_arg,
                            flags_result,
                            aggregate_arg,
                            aggregate_result,
                            typedef_inout,
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
                        let tuple_arg = *_instance
                            .get_typed_func::<
                                ((char, u32),),
                                (),
                            >(&mut store, &self.tuple_arg)?
                            .func();
                        let tuple_result = *_instance
                            .get_typed_func::<
                                (),
                                ((char, u32),),
                            >(&mut store, &self.tuple_result)?
                            .func();
                        let empty_arg = *_instance
                            .get_typed_func::<(Empty,), ()>(&mut store, &self.empty_arg)?
                            .func();
                        let empty_result = *_instance
                            .get_typed_func::<
                                (),
                                (Empty,),
                            >(&mut store, &self.empty_result)?
                            .func();
                        let scalar_arg = *_instance
                            .get_typed_func::<
                                (Scalars,),
                                (),
                            >(&mut store, &self.scalar_arg)?
                            .func();
                        let scalar_result = *_instance
                            .get_typed_func::<
                                (),
                                (Scalars,),
                            >(&mut store, &self.scalar_result)?
                            .func();
                        let flags_arg = *_instance
                            .get_typed_func::<
                                (ReallyFlags,),
                                (),
                            >(&mut store, &self.flags_arg)?
                            .func();
                        let flags_result = *_instance
                            .get_typed_func::<
                                (),
                                (ReallyFlags,),
                            >(&mut store, &self.flags_result)?
                            .func();
                        let aggregate_arg = *_instance
                            .get_typed_func::<
                                (&Aggregates,),
                                (),
                            >(&mut store, &self.aggregate_arg)?
                            .func();
                        let aggregate_result = *_instance
                            .get_typed_func::<
                                (),
                                (Aggregates,),
                            >(&mut store, &self.aggregate_result)?
                            .func();
                        let typedef_inout = *_instance
                            .get_typed_func::<
                                (TupleTypedef2,),
                                (i32,),
                            >(&mut store, &self.typedef_inout)?
                            .func();
                        Ok(Guest {
                            tuple_arg,
                            tuple_result,
                            empty_arg,
                            empty_result,
                            scalar_arg,
                            scalar_result,
                            flags_arg,
                            flags_result,
                            aggregate_arg,
                            aggregate_result,
                            typedef_inout,
                        })
                    }
                }
                impl Guest {
                    pub async fn call_tuple_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: (char, u32),
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                ((char, u32),),
                                (),
                            >::new_unchecked(self.tuple_arg)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_tuple_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<(char, u32)>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((char, u32),),
                            >::new_unchecked(self.tuple_result)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_empty_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Empty,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Empty,),
                                (),
                            >::new_unchecked(self.empty_arg)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_empty_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<Empty>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Empty,),
                            >::new_unchecked(self.empty_result)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_scalar_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Scalars,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Scalars,),
                                (),
                            >::new_unchecked(self.scalar_arg)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_scalar_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<Scalars>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Scalars,),
                            >::new_unchecked(self.scalar_result)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_flags_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: ReallyFlags,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (ReallyFlags,),
                                (),
                            >::new_unchecked(self.flags_arg)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_flags_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<ReallyFlags>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (ReallyFlags,),
                            >::new_unchecked(self.flags_result)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_aggregate_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Aggregates,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Aggregates,),
                                (),
                            >::new_unchecked(self.aggregate_arg)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_aggregate_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<Aggregates>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Aggregates,),
                            >::new_unchecked(self.aggregate_result)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_typedef_inout<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: TupleTypedef2,
                    ) -> wasmtime::Result<wasmtime::component::Promise<i32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (TupleTypedef2,),
                                (i32,),
                            >::new_unchecked(self.typedef_inout)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                }
            }
        }
    }
}
