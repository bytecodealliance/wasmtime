pub struct MyWorld {
    interface0: exports::foo::foo::variants::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl MyWorld {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::variants::Host + Send,
            T: Send,
        {
            foo::foo::variants::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::variants::Guest::new(
                &mut __exports
                    .instance("foo:foo/variants")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/variants` not present"
                        )
                    })?,
            )?;
            Ok(MyWorld { interface0 })
        }
        pub fn foo_foo_variants(&self) -> &exports::foo::foo::variants::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod variants {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(enum)]
            #[derive(Clone, Copy, Eq, PartialEq)]
            pub enum E1 {
                #[component(name = "a")]
                A,
            }
            impl core::fmt::Debug for E1 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        E1::A => f.debug_tuple("E1::A").finish(),
                    }
                }
            }
            const _: () = {
                assert!(1 == < E1 as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < E1 as wasmtime::component::ComponentType >::ALIGN32);
            };
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
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum V1 {
                #[component(name = "a")]
                A,
                #[component(name = "c")]
                C(E1),
                #[component(name = "d")]
                D(wasmtime::component::__internal::String),
                #[component(name = "e")]
                E(Empty),
                #[component(name = "f")]
                F,
                #[component(name = "g")]
                G(u32),
            }
            impl core::fmt::Debug for V1 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        V1::A => f.debug_tuple("V1::A").finish(),
                        V1::C(e) => f.debug_tuple("V1::C").field(e).finish(),
                        V1::D(e) => f.debug_tuple("V1::D").field(e).finish(),
                        V1::E(e) => f.debug_tuple("V1::E").field(e).finish(),
                        V1::F => f.debug_tuple("V1::F").finish(),
                        V1::G(e) => f.debug_tuple("V1::G").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(12 == < V1 as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < V1 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum Casts1 {
                #[component(name = "a")]
                A(i32),
                #[component(name = "b")]
                B(f32),
            }
            impl core::fmt::Debug for Casts1 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Casts1::A(e) => f.debug_tuple("Casts1::A").field(e).finish(),
                        Casts1::B(e) => f.debug_tuple("Casts1::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(8 == < Casts1 as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Casts1 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum Casts2 {
                #[component(name = "a")]
                A(f64),
                #[component(name = "b")]
                B(f32),
            }
            impl core::fmt::Debug for Casts2 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Casts2::A(e) => f.debug_tuple("Casts2::A").field(e).finish(),
                        Casts2::B(e) => f.debug_tuple("Casts2::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(16 == < Casts2 as wasmtime::component::ComponentType >::SIZE32);
                assert!(8 == < Casts2 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum Casts3 {
                #[component(name = "a")]
                A(f64),
                #[component(name = "b")]
                B(u64),
            }
            impl core::fmt::Debug for Casts3 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Casts3::A(e) => f.debug_tuple("Casts3::A").field(e).finish(),
                        Casts3::B(e) => f.debug_tuple("Casts3::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(16 == < Casts3 as wasmtime::component::ComponentType >::SIZE32);
                assert!(8 == < Casts3 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum Casts4 {
                #[component(name = "a")]
                A(u32),
                #[component(name = "b")]
                B(i64),
            }
            impl core::fmt::Debug for Casts4 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Casts4::A(e) => f.debug_tuple("Casts4::A").field(e).finish(),
                        Casts4::B(e) => f.debug_tuple("Casts4::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(16 == < Casts4 as wasmtime::component::ComponentType >::SIZE32);
                assert!(8 == < Casts4 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum Casts5 {
                #[component(name = "a")]
                A(f32),
                #[component(name = "b")]
                B(i64),
            }
            impl core::fmt::Debug for Casts5 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Casts5::A(e) => f.debug_tuple("Casts5::A").field(e).finish(),
                        Casts5::B(e) => f.debug_tuple("Casts5::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(16 == < Casts5 as wasmtime::component::ComponentType >::SIZE32);
                assert!(8 == < Casts5 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum Casts6 {
                #[component(name = "a")]
                A((f32, u32)),
                #[component(name = "b")]
                B((u32, u32)),
            }
            impl core::fmt::Debug for Casts6 {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Casts6::A(e) => f.debug_tuple("Casts6::A").field(e).finish(),
                        Casts6::B(e) => f.debug_tuple("Casts6::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(12 == < Casts6 as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Casts6 as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(enum)]
            #[derive(Clone, Copy, Eq, PartialEq)]
            pub enum MyErrno {
                #[component(name = "bad1")]
                Bad1,
                #[component(name = "bad2")]
                Bad2,
            }
            impl MyErrno {
                pub fn name(&self) -> &'static str {
                    match self {
                        MyErrno::Bad1 => "bad1",
                        MyErrno::Bad2 => "bad2",
                    }
                }
                pub fn message(&self) -> &'static str {
                    match self {
                        MyErrno::Bad1 => "",
                        MyErrno::Bad2 => "",
                    }
                }
            }
            impl core::fmt::Debug for MyErrno {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("MyErrno")
                        .field("code", &(*self as i32))
                        .field("name", &self.name())
                        .field("message", &self.message())
                        .finish()
                }
            }
            impl core::fmt::Display for MyErrno {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{} (error {})", self.name(), * self as i32)
                }
            }
            impl std::error::Error for MyErrno {}
            const _: () = {
                assert!(1 == < MyErrno as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < MyErrno as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct IsClone {
                #[component(name = "v1")]
                pub v1: V1,
            }
            impl core::fmt::Debug for IsClone {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("IsClone").field("v1", &self.v1).finish()
                }
            }
            const _: () = {
                assert!(12 == < IsClone as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < IsClone as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host {
                async fn e1_arg(&mut self, x: E1) -> ();
                async fn e1_result(&mut self) -> E1;
                async fn v1_arg(&mut self, x: V1) -> ();
                async fn v1_result(&mut self) -> V1;
                async fn bool_arg(&mut self, x: bool) -> ();
                async fn bool_result(&mut self) -> bool;
                async fn option_arg(
                    &mut self,
                    a: Option<bool>,
                    b: Option<()>,
                    c: Option<u32>,
                    d: Option<E1>,
                    e: Option<f32>,
                    g: Option<Option<bool>>,
                ) -> ();
                async fn option_result(
                    &mut self,
                ) -> (
                    Option<bool>,
                    Option<()>,
                    Option<u32>,
                    Option<E1>,
                    Option<f32>,
                    Option<Option<bool>>,
                );
                async fn casts(
                    &mut self,
                    a: Casts1,
                    b: Casts2,
                    c: Casts3,
                    d: Casts4,
                    e: Casts5,
                    f: Casts6,
                ) -> (Casts1, Casts2, Casts3, Casts4, Casts5, Casts6);
                async fn result_arg(
                    &mut self,
                    a: Result<(), ()>,
                    b: Result<(), E1>,
                    c: Result<E1, ()>,
                    d: Result<(), ()>,
                    e: Result<u32, V1>,
                    f: Result<
                        wasmtime::component::__internal::String,
                        wasmtime::component::__internal::Vec<u8>,
                    >,
                ) -> ();
                async fn result_result(
                    &mut self,
                ) -> (
                    Result<(), ()>,
                    Result<(), E1>,
                    Result<E1, ()>,
                    Result<(), ()>,
                    Result<u32, V1>,
                    Result<
                        wasmtime::component::__internal::String,
                        wasmtime::component::__internal::Vec<u8>,
                    >,
                );
                async fn return_result_sugar(&mut self) -> Result<i32, MyErrno>;
                async fn return_result_sugar2(&mut self) -> Result<(), MyErrno>;
                async fn return_result_sugar3(&mut self) -> Result<MyErrno, MyErrno>;
                async fn return_result_sugar4(&mut self) -> Result<(i32, u32), MyErrno>;
                async fn return_option_sugar(&mut self) -> Option<i32>;
                async fn return_option_sugar2(&mut self) -> Option<MyErrno>;
                async fn result_simple(&mut self) -> Result<u32, i32>;
                async fn is_clone_arg(&mut self, a: IsClone) -> ();
                async fn is_clone_return(&mut self) -> IsClone;
                async fn return_named_option(&mut self) -> Option<u8>;
                async fn return_named_result(&mut self) -> Result<u8, MyErrno>;
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                T: Send,
                U: Host + Send,
            {
                let mut inst = linker.instance("foo:foo/variants")?;
                inst.func_wrap_async(
                    "e1-arg",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (E1,)| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::e1_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "e1-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::e1_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "v1-arg",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (V1,)| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::v1_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "v1-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::v1_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "bool-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (bool,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::bool_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "bool-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::bool_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "option-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                        ): (
                            Option<bool>,
                            Option<()>,
                            Option<u32>,
                            Option<E1>,
                            Option<f32>,
                            Option<Option<bool>>,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::option_arg(
                                host,
                                arg0,
                                arg1,
                                arg2,
                                arg3,
                                arg4,
                                arg5,
                            )
                            .await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "option-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::option_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "casts",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                        ): (Casts1, Casts2, Casts3, Casts4, Casts5, Casts6)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::casts(host, arg0, arg1, arg2, arg3, arg4, arg5)
                            .await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "result-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                        ): (
                            Result<(), ()>,
                            Result<(), E1>,
                            Result<E1, ()>,
                            Result<(), ()>,
                            Result<u32, V1>,
                            Result<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::Vec<u8>,
                            >,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::result_arg(
                                host,
                                arg0,
                                arg1,
                                arg2,
                                arg3,
                                arg4,
                                arg5,
                            )
                            .await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "result-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::result_result(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-result-sugar",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_result_sugar(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-result-sugar2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_result_sugar2(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-result-sugar3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_result_sugar3(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-result-sugar4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_result_sugar4(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-option-sugar",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_option_sugar(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-option-sugar2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_option_sugar2(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "result-simple",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::result_simple(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "is-clone-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (IsClone,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::is_clone_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "is-clone-return",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::is_clone_return(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-named-option",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_named_option(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "return-named-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::return_named_result(host).await;
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
            pub mod variants {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(enum)]
                #[derive(Clone, Copy, Eq, PartialEq)]
                pub enum E1 {
                    #[component(name = "a")]
                    A,
                }
                impl core::fmt::Debug for E1 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            E1::A => f.debug_tuple("E1::A").finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(1 == < E1 as wasmtime::component::ComponentType >::SIZE32);
                    assert!(1 == < E1 as wasmtime::component::ComponentType >::ALIGN32);
                };
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
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone)]
                pub enum V1 {
                    #[component(name = "a")]
                    A,
                    #[component(name = "c")]
                    C(E1),
                    #[component(name = "d")]
                    D(wasmtime::component::__internal::String),
                    #[component(name = "e")]
                    E(Empty),
                    #[component(name = "f")]
                    F,
                    #[component(name = "g")]
                    G(u32),
                }
                impl core::fmt::Debug for V1 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            V1::A => f.debug_tuple("V1::A").finish(),
                            V1::C(e) => f.debug_tuple("V1::C").field(e).finish(),
                            V1::D(e) => f.debug_tuple("V1::D").field(e).finish(),
                            V1::E(e) => f.debug_tuple("V1::E").field(e).finish(),
                            V1::F => f.debug_tuple("V1::F").finish(),
                            V1::G(e) => f.debug_tuple("V1::G").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(12 == < V1 as wasmtime::component::ComponentType >::SIZE32);
                    assert!(4 == < V1 as wasmtime::component::ComponentType >::ALIGN32);
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone, Copy)]
                pub enum Casts1 {
                    #[component(name = "a")]
                    A(i32),
                    #[component(name = "b")]
                    B(f32),
                }
                impl core::fmt::Debug for Casts1 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Casts1::A(e) => f.debug_tuple("Casts1::A").field(e).finish(),
                            Casts1::B(e) => f.debug_tuple("Casts1::B").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(
                        8 == < Casts1 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < Casts1 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone, Copy)]
                pub enum Casts2 {
                    #[component(name = "a")]
                    A(f64),
                    #[component(name = "b")]
                    B(f32),
                }
                impl core::fmt::Debug for Casts2 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Casts2::A(e) => f.debug_tuple("Casts2::A").field(e).finish(),
                            Casts2::B(e) => f.debug_tuple("Casts2::B").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(
                        16 == < Casts2 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        8 == < Casts2 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone, Copy)]
                pub enum Casts3 {
                    #[component(name = "a")]
                    A(f64),
                    #[component(name = "b")]
                    B(u64),
                }
                impl core::fmt::Debug for Casts3 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Casts3::A(e) => f.debug_tuple("Casts3::A").field(e).finish(),
                            Casts3::B(e) => f.debug_tuple("Casts3::B").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(
                        16 == < Casts3 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        8 == < Casts3 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone, Copy)]
                pub enum Casts4 {
                    #[component(name = "a")]
                    A(u32),
                    #[component(name = "b")]
                    B(i64),
                }
                impl core::fmt::Debug for Casts4 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Casts4::A(e) => f.debug_tuple("Casts4::A").field(e).finish(),
                            Casts4::B(e) => f.debug_tuple("Casts4::B").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(
                        16 == < Casts4 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        8 == < Casts4 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone, Copy)]
                pub enum Casts5 {
                    #[component(name = "a")]
                    A(f32),
                    #[component(name = "b")]
                    B(i64),
                }
                impl core::fmt::Debug for Casts5 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Casts5::A(e) => f.debug_tuple("Casts5::A").field(e).finish(),
                            Casts5::B(e) => f.debug_tuple("Casts5::B").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(
                        16 == < Casts5 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        8 == < Casts5 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone, Copy)]
                pub enum Casts6 {
                    #[component(name = "a")]
                    A((f32, u32)),
                    #[component(name = "b")]
                    B((u32, u32)),
                }
                impl core::fmt::Debug for Casts6 {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            Casts6::A(e) => f.debug_tuple("Casts6::A").field(e).finish(),
                            Casts6::B(e) => f.debug_tuple("Casts6::B").field(e).finish(),
                        }
                    }
                }
                const _: () = {
                    assert!(
                        12 == < Casts6 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < Casts6 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(enum)]
                #[derive(Clone, Copy, Eq, PartialEq)]
                pub enum MyErrno {
                    #[component(name = "bad1")]
                    Bad1,
                    #[component(name = "bad2")]
                    Bad2,
                }
                impl MyErrno {
                    pub fn name(&self) -> &'static str {
                        match self {
                            MyErrno::Bad1 => "bad1",
                            MyErrno::Bad2 => "bad2",
                        }
                    }
                    pub fn message(&self) -> &'static str {
                        match self {
                            MyErrno::Bad1 => "",
                            MyErrno::Bad2 => "",
                        }
                    }
                }
                impl core::fmt::Debug for MyErrno {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("MyErrno")
                            .field("code", &(*self as i32))
                            .field("name", &self.name())
                            .field("message", &self.message())
                            .finish()
                    }
                }
                impl core::fmt::Display for MyErrno {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        write!(f, "{} (error {})", self.name(), * self as i32)
                    }
                }
                impl std::error::Error for MyErrno {}
                const _: () = {
                    assert!(
                        1 == < MyErrno as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < MyErrno as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct IsClone {
                    #[component(name = "v1")]
                    pub v1: V1,
                }
                impl core::fmt::Debug for IsClone {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("IsClone").field("v1", &self.v1).finish()
                    }
                }
                const _: () = {
                    assert!(
                        12 == < IsClone as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < IsClone as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                pub struct Guest {
                    e1_arg: wasmtime::component::Func,
                    e1_result: wasmtime::component::Func,
                    v1_arg: wasmtime::component::Func,
                    v1_result: wasmtime::component::Func,
                    bool_arg: wasmtime::component::Func,
                    bool_result: wasmtime::component::Func,
                    option_arg: wasmtime::component::Func,
                    option_result: wasmtime::component::Func,
                    casts: wasmtime::component::Func,
                    result_arg: wasmtime::component::Func,
                    result_result: wasmtime::component::Func,
                    return_result_sugar: wasmtime::component::Func,
                    return_result_sugar2: wasmtime::component::Func,
                    return_result_sugar3: wasmtime::component::Func,
                    return_result_sugar4: wasmtime::component::Func,
                    return_option_sugar: wasmtime::component::Func,
                    return_option_sugar2: wasmtime::component::Func,
                    result_simple: wasmtime::component::Func,
                    is_clone_arg: wasmtime::component::Func,
                    is_clone_return: wasmtime::component::Func,
                    return_named_option: wasmtime::component::Func,
                    return_named_result: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let e1_arg = *__exports
                            .typed_func::<(E1,), ()>("e1-arg")?
                            .func();
                        let e1_result = *__exports
                            .typed_func::<(), (E1,)>("e1-result")?
                            .func();
                        let v1_arg = *__exports
                            .typed_func::<(&V1,), ()>("v1-arg")?
                            .func();
                        let v1_result = *__exports
                            .typed_func::<(), (V1,)>("v1-result")?
                            .func();
                        let bool_arg = *__exports
                            .typed_func::<(bool,), ()>("bool-arg")?
                            .func();
                        let bool_result = *__exports
                            .typed_func::<(), (bool,)>("bool-result")?
                            .func();
                        let option_arg = *__exports
                            .typed_func::<
                                (
                                    Option<bool>,
                                    Option<()>,
                                    Option<u32>,
                                    Option<E1>,
                                    Option<f32>,
                                    Option<Option<bool>>,
                                ),
                                (),
                            >("option-arg")?
                            .func();
                        let option_result = *__exports
                            .typed_func::<
                                (),
                                (
                                    (
                                        Option<bool>,
                                        Option<()>,
                                        Option<u32>,
                                        Option<E1>,
                                        Option<f32>,
                                        Option<Option<bool>>,
                                    ),
                                ),
                            >("option-result")?
                            .func();
                        let casts = *__exports
                            .typed_func::<
                                (Casts1, Casts2, Casts3, Casts4, Casts5, Casts6),
                                ((Casts1, Casts2, Casts3, Casts4, Casts5, Casts6),),
                            >("casts")?
                            .func();
                        let result_arg = *__exports
                            .typed_func::<
                                (
                                    Result<(), ()>,
                                    Result<(), E1>,
                                    Result<E1, ()>,
                                    Result<(), ()>,
                                    Result<u32, &V1>,
                                    Result<&str, &[u8]>,
                                ),
                                (),
                            >("result-arg")?
                            .func();
                        let result_result = *__exports
                            .typed_func::<
                                (),
                                (
                                    (
                                        Result<(), ()>,
                                        Result<(), E1>,
                                        Result<E1, ()>,
                                        Result<(), ()>,
                                        Result<u32, V1>,
                                        Result<
                                            wasmtime::component::__internal::String,
                                            wasmtime::component::__internal::Vec<u8>,
                                        >,
                                    ),
                                ),
                            >("result-result")?
                            .func();
                        let return_result_sugar = *__exports
                            .typed_func::<
                                (),
                                (Result<i32, MyErrno>,),
                            >("return-result-sugar")?
                            .func();
                        let return_result_sugar2 = *__exports
                            .typed_func::<
                                (),
                                (Result<(), MyErrno>,),
                            >("return-result-sugar2")?
                            .func();
                        let return_result_sugar3 = *__exports
                            .typed_func::<
                                (),
                                (Result<MyErrno, MyErrno>,),
                            >("return-result-sugar3")?
                            .func();
                        let return_result_sugar4 = *__exports
                            .typed_func::<
                                (),
                                (Result<(i32, u32), MyErrno>,),
                            >("return-result-sugar4")?
                            .func();
                        let return_option_sugar = *__exports
                            .typed_func::<(), (Option<i32>,)>("return-option-sugar")?
                            .func();
                        let return_option_sugar2 = *__exports
                            .typed_func::<
                                (),
                                (Option<MyErrno>,),
                            >("return-option-sugar2")?
                            .func();
                        let result_simple = *__exports
                            .typed_func::<(), (Result<u32, i32>,)>("result-simple")?
                            .func();
                        let is_clone_arg = *__exports
                            .typed_func::<(&IsClone,), ()>("is-clone-arg")?
                            .func();
                        let is_clone_return = *__exports
                            .typed_func::<(), (IsClone,)>("is-clone-return")?
                            .func();
                        let return_named_option = *__exports
                            .typed_func::<(), (Option<u8>,)>("return-named-option")?
                            .func();
                        let return_named_result = *__exports
                            .typed_func::<
                                (),
                                (Result<u8, MyErrno>,),
                            >("return-named-result")?
                            .func();
                        Ok(Guest {
                            e1_arg,
                            e1_result,
                            v1_arg,
                            v1_result,
                            bool_arg,
                            bool_result,
                            option_arg,
                            option_result,
                            casts,
                            result_arg,
                            result_result,
                            return_result_sugar,
                            return_result_sugar2,
                            return_result_sugar3,
                            return_result_sugar4,
                            return_option_sugar,
                            return_option_sugar2,
                            result_simple,
                            is_clone_arg,
                            is_clone_return,
                            return_named_option,
                            return_named_result,
                        })
                    }
                    pub async fn call_e1_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: E1,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (E1,),
                                (),
                            >::new_unchecked(self.e1_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_e1_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<E1>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (E1,),
                            >::new_unchecked(self.e1_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_v1_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &V1,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&V1,),
                                (),
                            >::new_unchecked(self.v1_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_v1_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<V1>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (V1,),
                            >::new_unchecked(self.v1_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_bool_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: bool,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (bool,),
                                (),
                            >::new_unchecked(self.bool_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_bool_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<bool>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (bool,),
                            >::new_unchecked(self.bool_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_option_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Option<bool>,
                        arg1: Option<()>,
                        arg2: Option<u32>,
                        arg3: Option<E1>,
                        arg4: Option<f32>,
                        arg5: Option<Option<bool>>,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    Option<bool>,
                                    Option<()>,
                                    Option<u32>,
                                    Option<E1>,
                                    Option<f32>,
                                    Option<Option<bool>>,
                                ),
                                (),
                            >::new_unchecked(self.option_arg)
                        };
                        let () = callee
                            .call_async(
                                store.as_context_mut(),
                                (arg0, arg1, arg2, arg3, arg4, arg5),
                            )
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_option_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<
                        (
                            Option<bool>,
                            Option<()>,
                            Option<u32>,
                            Option<E1>,
                            Option<f32>,
                            Option<Option<bool>>,
                        ),
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (
                                    (
                                        Option<bool>,
                                        Option<()>,
                                        Option<u32>,
                                        Option<E1>,
                                        Option<f32>,
                                        Option<Option<bool>>,
                                    ),
                                ),
                            >::new_unchecked(self.option_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_casts<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Casts1,
                        arg1: Casts2,
                        arg2: Casts3,
                        arg3: Casts4,
                        arg4: Casts5,
                        arg5: Casts6,
                    ) -> wasmtime::Result<
                        (Casts1, Casts2, Casts3, Casts4, Casts5, Casts6),
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Casts1, Casts2, Casts3, Casts4, Casts5, Casts6),
                                ((Casts1, Casts2, Casts3, Casts4, Casts5, Casts6),),
                            >::new_unchecked(self.casts)
                        };
                        let (ret0,) = callee
                            .call_async(
                                store.as_context_mut(),
                                (arg0, arg1, arg2, arg3, arg4, arg5),
                            )
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_result_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Result<(), ()>,
                        arg1: Result<(), E1>,
                        arg2: Result<E1, ()>,
                        arg3: Result<(), ()>,
                        arg4: Result<u32, &V1>,
                        arg5: Result<&str, &[u8]>,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    Result<(), ()>,
                                    Result<(), E1>,
                                    Result<E1, ()>,
                                    Result<(), ()>,
                                    Result<u32, &V1>,
                                    Result<&str, &[u8]>,
                                ),
                                (),
                            >::new_unchecked(self.result_arg)
                        };
                        let () = callee
                            .call_async(
                                store.as_context_mut(),
                                (arg0, arg1, arg2, arg3, arg4, arg5),
                            )
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_result_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<
                        (
                            Result<(), ()>,
                            Result<(), E1>,
                            Result<E1, ()>,
                            Result<(), ()>,
                            Result<u32, V1>,
                            Result<
                                wasmtime::component::__internal::String,
                                wasmtime::component::__internal::Vec<u8>,
                            >,
                        ),
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (
                                    (
                                        Result<(), ()>,
                                        Result<(), E1>,
                                        Result<E1, ()>,
                                        Result<(), ()>,
                                        Result<u32, V1>,
                                        Result<
                                            wasmtime::component::__internal::String,
                                            wasmtime::component::__internal::Vec<u8>,
                                        >,
                                    ),
                                ),
                            >::new_unchecked(self.result_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_result_sugar<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Result<i32, MyErrno>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Result<i32, MyErrno>,),
                            >::new_unchecked(self.return_result_sugar)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_result_sugar2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Result<(), MyErrno>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Result<(), MyErrno>,),
                            >::new_unchecked(self.return_result_sugar2)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_result_sugar3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Result<MyErrno, MyErrno>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Result<MyErrno, MyErrno>,),
                            >::new_unchecked(self.return_result_sugar3)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_result_sugar4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Result<(i32, u32), MyErrno>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Result<(i32, u32), MyErrno>,),
                            >::new_unchecked(self.return_result_sugar4)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_option_sugar<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Option<i32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Option<i32>,),
                            >::new_unchecked(self.return_option_sugar)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_option_sugar2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Option<MyErrno>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Option<MyErrno>,),
                            >::new_unchecked(self.return_option_sugar2)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_result_simple<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Result<u32, i32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Result<u32, i32>,),
                            >::new_unchecked(self.result_simple)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_is_clone_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &IsClone,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&IsClone,),
                                (),
                            >::new_unchecked(self.is_clone_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_is_clone_return<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<IsClone>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (IsClone,),
                            >::new_unchecked(self.is_clone_return)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_named_option<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Option<u8>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Option<u8>,),
                            >::new_unchecked(self.return_named_option)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_return_named_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<Result<u8, MyErrno>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (Result<u8, MyErrno>,),
                            >::new_unchecked(self.return_named_result)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
