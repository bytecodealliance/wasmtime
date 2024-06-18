/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::foo::foo::records::GuestPre,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure is created through either
/// [`TheWorld::instantiate_async`] or by first creating
/// a [`TheWorldPre`] followed by using
/// [`TheWorldPre::instantiate_async`].
pub struct TheWorld {
    interface0: exports::foo::foo::records::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> TheWorldPre<_T> {
        /// Creates a new copy of `TheWorldPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::foo::foo::records::GuestPre::new(_component)?;
            Ok(TheWorldPre {
                instance_pre,
                interface0,
            })
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
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
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
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::records::Host + Send,
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
            use wasmtime::component::__internal::anyhow;
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
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn tuple_arg(&mut self, x: (char, u32)) -> ();
                async fn tuple_result(&mut self) -> (char, u32);
                async fn empty_arg(&mut self, x: Empty) -> ();
                async fn empty_result(&mut self) -> Empty;
                async fn scalar_arg(&mut self, x: Scalars) -> ();
                async fn scalar_result(&mut self) -> Scalars;
                async fn flags_arg(&mut self, x: ReallyFlags) -> ();
                async fn flags_result(&mut self) -> ReallyFlags;
                async fn aggregate_arg(&mut self, x: Aggregates) -> ();
                async fn aggregate_result(&mut self) -> Aggregates;
                async fn typedef_inout(&mut self, e: TupleTypedef2) -> i32;
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
                let mut inst = linker.instance("foo:foo/records")?;
                inst.func_wrap_async(
                    "tuple-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): ((char, u32),)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::tuple_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "tuple-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::tuple_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "empty-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Empty,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::empty_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "empty-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::empty_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "scalar-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Scalars,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::scalar_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "scalar-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::scalar_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "flags-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (ReallyFlags,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::flags_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "flags-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::flags_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "aggregate-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Aggregates,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::aggregate_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "aggregate-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::aggregate_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "typedef-inout",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (TupleTypedef2,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::typedef_inout(host, arg0).await;
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
                async fn tuple_arg(&mut self, x: (char, u32)) -> () {
                    Host::tuple_arg(*self, x).await
                }
                async fn tuple_result(&mut self) -> (char, u32) {
                    Host::tuple_result(*self).await
                }
                async fn empty_arg(&mut self, x: Empty) -> () {
                    Host::empty_arg(*self, x).await
                }
                async fn empty_result(&mut self) -> Empty {
                    Host::empty_result(*self).await
                }
                async fn scalar_arg(&mut self, x: Scalars) -> () {
                    Host::scalar_arg(*self, x).await
                }
                async fn scalar_result(&mut self) -> Scalars {
                    Host::scalar_result(*self).await
                }
                async fn flags_arg(&mut self, x: ReallyFlags) -> () {
                    Host::flags_arg(*self, x).await
                }
                async fn flags_result(&mut self) -> ReallyFlags {
                    Host::flags_result(*self).await
                }
                async fn aggregate_arg(&mut self, x: Aggregates) -> () {
                    Host::aggregate_arg(*self, x).await
                }
                async fn aggregate_result(&mut self) -> Aggregates {
                    Host::aggregate_result(*self).await
                }
                async fn typedef_inout(&mut self, e: TupleTypedef2) -> i32 {
                    Host::typedef_inout(*self, e).await
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
                use wasmtime::component::__internal::anyhow;
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
                pub struct GuestPre {
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
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/records")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/records`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/records` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let tuple_arg = _lookup("tuple-arg")?;
                        let tuple_result = _lookup("tuple-result")?;
                        let empty_arg = _lookup("empty-arg")?;
                        let empty_result = _lookup("empty-result")?;
                        let scalar_arg = _lookup("scalar-arg")?;
                        let scalar_result = _lookup("scalar-result")?;
                        let flags_arg = _lookup("flags-arg")?;
                        let flags_result = _lookup("flags-result")?;
                        let aggregate_arg = _lookup("aggregate-arg")?;
                        let aggregate_result = _lookup("aggregate-result")?;
                        let typedef_inout = _lookup("typedef-inout")?;
                        Ok(GuestPre {
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
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                ((char, u32),),
                                (),
                            >::new_unchecked(self.tuple_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_tuple_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(char, u32)>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((char, u32),),
                            >::new_unchecked(self.tuple_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_empty_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Empty,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Empty,),
                                (),
                            >::new_unchecked(self.empty_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_empty_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Empty>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Empty,),
                            >::new_unchecked(self.empty_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_scalar_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Scalars,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Scalars,),
                                (),
                            >::new_unchecked(self.scalar_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_scalar_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Scalars>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Scalars,),
                            >::new_unchecked(self.scalar_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_flags_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: ReallyFlags,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (ReallyFlags,),
                                (),
                            >::new_unchecked(self.flags_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_flags_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<ReallyFlags>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (ReallyFlags,),
                            >::new_unchecked(self.flags_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_aggregate_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &Aggregates,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&Aggregates,),
                                (),
                            >::new_unchecked(self.aggregate_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_aggregate_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Aggregates>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Aggregates,),
                            >::new_unchecked(self.aggregate_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_typedef_inout<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: TupleTypedef2,
                    ) -> wasmtime::Result<i32>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (TupleTypedef2,),
                                (i32,),
                            >::new_unchecked(self.typedef_inout)
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
