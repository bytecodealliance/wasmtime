pub struct TheWorld {
    interface0: exports::foo::foo::records::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorld {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::records::Host + Send,
            T: Send,
        {
            foo::foo::records::add_to_linker(linker, get)?;
            Ok(())
        }
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub async fn instantiate_async<T: Send>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate_async(&mut store, component).await?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Instantiates a pre-instantiated module using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub async fn instantiate_pre<T: Send>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance_pre: &wasmtime::component::InstancePre<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = instance_pre.instantiate_async(&mut store).await?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Low-level creation wrapper for wrapping up the exports
        /// of the `instance` provided in this structure of wasm
        /// exports.
        ///
        /// This function will extract exports from the `instance`
        /// defined within `store` and wrap them all up in the
        /// returned structure which can be used to interact with
        /// the wasm module.
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let mut store = store.as_context_mut();
            let mut exports = instance.exports(&mut store);
            let mut __exports = exports.root();
            let interface0 = exports::foo::foo::records::Guest::new(
                &mut __exports
                    .instance("foo:foo/records")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/records` not present"
                        )
                    })?,
            )?;
            Ok(TheWorld { interface0 })
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
            pub trait Host {
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
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                T: Send,
                U: Host + Send,
            {
                let mut inst = linker.instance("foo:foo/records")?;
                inst.func_wrap_async(
                    "tuple-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): ((char, u32),)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::tuple_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "tuple-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
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
                        let host = get(caller.data_mut());
                        let r = Host::empty_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "empty-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
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
                        let host = get(caller.data_mut());
                        let r = Host::scalar_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "scalar-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
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
                        let host = get(caller.data_mut());
                        let r = Host::flags_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "flags-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
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
                        let host = get(caller.data_mut());
                        let r = Host::aggregate_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "aggregate-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
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
                        let host = get(caller.data_mut());
                        let r = Host::typedef_inout(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                Ok(())
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
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let tuple_arg = *__exports
                            .typed_func::<((char, u32),), ()>("tuple-arg")?
                            .func();
                        let tuple_result = *__exports
                            .typed_func::<(), ((char, u32),)>("tuple-result")?
                            .func();
                        let empty_arg = *__exports
                            .typed_func::<(Empty,), ()>("empty-arg")?
                            .func();
                        let empty_result = *__exports
                            .typed_func::<(), (Empty,)>("empty-result")?
                            .func();
                        let scalar_arg = *__exports
                            .typed_func::<(Scalars,), ()>("scalar-arg")?
                            .func();
                        let scalar_result = *__exports
                            .typed_func::<(), (Scalars,)>("scalar-result")?
                            .func();
                        let flags_arg = *__exports
                            .typed_func::<(ReallyFlags,), ()>("flags-arg")?
                            .func();
                        let flags_result = *__exports
                            .typed_func::<(), (ReallyFlags,)>("flags-result")?
                            .func();
                        let aggregate_arg = *__exports
                            .typed_func::<(&Aggregates,), ()>("aggregate-arg")?
                            .func();
                        let aggregate_result = *__exports
                            .typed_func::<(), (Aggregates,)>("aggregate-result")?
                            .func();
                        let typedef_inout = *__exports
                            .typed_func::<(TupleTypedef2,), (i32,)>("typedef-inout")?
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
