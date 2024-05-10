pub struct TheWorld {
    interface0: exports::foo::foo::conventions::Guest,
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
            T: Send,
            U: foo::foo::conventions::Host + Send,
        {
            foo::foo::conventions::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::conventions::Guest::new(
                &mut __exports
                    .instance("foo:foo/conventions")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/conventions` not present"
                        )
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_conventions(&self) -> &exports::foo::foo::conventions::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod conventions {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct LudicrousSpeed {
                #[component(name = "how-fast-are-you-going")]
                pub how_fast_are_you_going: u32,
                #[component(name = "i-am-going-extremely-slow")]
                pub i_am_going_extremely_slow: u64,
            }
            impl core::fmt::Debug for LudicrousSpeed {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("LudicrousSpeed")
                        .field("how-fast-are-you-going", &self.how_fast_are_you_going)
                        .field(
                            "i-am-going-extremely-slow",
                            &self.i_am_going_extremely_slow,
                        )
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    16 == < LudicrousSpeed as wasmtime::component::ComponentType
                    >::SIZE32
                );
                assert!(
                    8 == < LudicrousSpeed as wasmtime::component::ComponentType
                    >::ALIGN32
                );
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn kebab_case(&mut self) -> ();
                async fn foo(&mut self, x: LudicrousSpeed) -> ();
                async fn function_with_dashes(&mut self) -> ();
                async fn function_with_no_weird_characters(&mut self) -> ();
                async fn apple(&mut self) -> ();
                async fn apple_pear(&mut self) -> ();
                async fn apple_pear_grape(&mut self) -> ();
                async fn a0(&mut self) -> ();
                /// Comment out identifiers that collide when mapped to snake_case, for now; see
                /// https://github.com/WebAssembly/component-model/issues/118
                /// APPLE: func()
                /// APPLE-pear-GRAPE: func()
                /// apple-PEAR-grape: func()
                async fn is_xml(&mut self) -> ();
                async fn explicit(&mut self) -> ();
                async fn explicit_kebab(&mut self) -> ();
                /// Identifiers with the same name as keywords are quoted.
                async fn bool(&mut self) -> ();
            }
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Output + Send + Sync + Copy + 'static {
                type Output: Host + Send;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host + Send,
            {
                type Output = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()>
            where
                T: Send,
            {
                let mut inst = linker.instance("foo:foo/conventions")?;
                inst.func_wrap_async(
                    "kebab-case",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::kebab_case(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "foo",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (LudicrousSpeed,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::foo(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "function-with-dashes",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::function_with_dashes(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "function-with-no-weird-characters",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::function_with_no_weird_characters(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "apple",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::apple(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "apple-pear",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::apple_pear(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "apple-pear-grape",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::apple_pear_grape(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a0",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a0(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "is-XML",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::is_xml(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "explicit",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::explicit(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "explicit-kebab",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::explicit_kebab(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "bool",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::bool(host).await;
                        Ok(r)
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
                async fn kebab_case(&mut self) -> () {
                    Host::kebab_case(*self).await
                }
                async fn foo(&mut self, x: LudicrousSpeed) -> () {
                    Host::foo(*self, x).await
                }
                async fn function_with_dashes(&mut self) -> () {
                    Host::function_with_dashes(*self).await
                }
                async fn function_with_no_weird_characters(&mut self) -> () {
                    Host::function_with_no_weird_characters(*self).await
                }
                async fn apple(&mut self) -> () {
                    Host::apple(*self).await
                }
                async fn apple_pear(&mut self) -> () {
                    Host::apple_pear(*self).await
                }
                async fn apple_pear_grape(&mut self) -> () {
                    Host::apple_pear_grape(*self).await
                }
                async fn a0(&mut self) -> () {
                    Host::a0(*self).await
                }
                /// Comment out identifiers that collide when mapped to snake_case, for now; see
                /// https://github.com/WebAssembly/component-model/issues/118
                /// APPLE: func()
                /// APPLE-pear-GRAPE: func()
                /// apple-PEAR-grape: func()
                async fn is_xml(&mut self) -> () {
                    Host::is_xml(*self).await
                }
                async fn explicit(&mut self) -> () {
                    Host::explicit(*self).await
                }
                async fn explicit_kebab(&mut self) -> () {
                    Host::explicit_kebab(*self).await
                }
                /// Identifiers with the same name as keywords are quoted.
                async fn bool(&mut self) -> () {
                    Host::bool(*self).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod conventions {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone, Copy)]
                pub struct LudicrousSpeed {
                    #[component(name = "how-fast-are-you-going")]
                    pub how_fast_are_you_going: u32,
                    #[component(name = "i-am-going-extremely-slow")]
                    pub i_am_going_extremely_slow: u64,
                }
                impl core::fmt::Debug for LudicrousSpeed {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("LudicrousSpeed")
                            .field(
                                "how-fast-are-you-going",
                                &self.how_fast_are_you_going,
                            )
                            .field(
                                "i-am-going-extremely-slow",
                                &self.i_am_going_extremely_slow,
                            )
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        16 == < LudicrousSpeed as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        8 == < LudicrousSpeed as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub struct Guest {
                    kebab_case: wasmtime::component::Func,
                    foo: wasmtime::component::Func,
                    function_with_dashes: wasmtime::component::Func,
                    function_with_no_weird_characters: wasmtime::component::Func,
                    apple: wasmtime::component::Func,
                    apple_pear: wasmtime::component::Func,
                    apple_pear_grape: wasmtime::component::Func,
                    a0: wasmtime::component::Func,
                    is_xml: wasmtime::component::Func,
                    explicit: wasmtime::component::Func,
                    explicit_kebab: wasmtime::component::Func,
                    bool: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let kebab_case = *__exports
                            .typed_func::<(), ()>("kebab-case")?
                            .func();
                        let foo = *__exports
                            .typed_func::<(LudicrousSpeed,), ()>("foo")?
                            .func();
                        let function_with_dashes = *__exports
                            .typed_func::<(), ()>("function-with-dashes")?
                            .func();
                        let function_with_no_weird_characters = *__exports
                            .typed_func::<(), ()>("function-with-no-weird-characters")?
                            .func();
                        let apple = *__exports.typed_func::<(), ()>("apple")?.func();
                        let apple_pear = *__exports
                            .typed_func::<(), ()>("apple-pear")?
                            .func();
                        let apple_pear_grape = *__exports
                            .typed_func::<(), ()>("apple-pear-grape")?
                            .func();
                        let a0 = *__exports.typed_func::<(), ()>("a0")?.func();
                        let is_xml = *__exports.typed_func::<(), ()>("is-XML")?.func();
                        let explicit = *__exports
                            .typed_func::<(), ()>("explicit")?
                            .func();
                        let explicit_kebab = *__exports
                            .typed_func::<(), ()>("explicit-kebab")?
                            .func();
                        let bool = *__exports.typed_func::<(), ()>("bool")?.func();
                        Ok(Guest {
                            kebab_case,
                            foo,
                            function_with_dashes,
                            function_with_no_weird_characters,
                            apple,
                            apple_pear,
                            apple_pear_grape,
                            a0,
                            is_xml,
                            explicit,
                            explicit_kebab,
                            bool,
                        })
                    }
                    pub async fn call_kebab_case<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.kebab_case)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_foo<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: LudicrousSpeed,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (LudicrousSpeed,),
                                (),
                            >::new_unchecked(self.foo)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_function_with_dashes<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.function_with_dashes)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_function_with_no_weird_characters<
                        S: wasmtime::AsContextMut,
                    >(&self, mut store: S) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.function_with_no_weird_characters)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_apple<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_apple_pear<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple_pear)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_apple_pear_grape<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple_pear_grape)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a0<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.a0)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    /// Comment out identifiers that collide when mapped to snake_case, for now; see
                    /// https://github.com/WebAssembly/component-model/issues/118
                    /// APPLE: func()
                    /// APPLE-pear-GRAPE: func()
                    /// apple-PEAR-grape: func()
                    pub async fn call_is_xml<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.is_xml)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_explicit<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.explicit)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_explicit_kebab<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.explicit_kebab)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    /// Identifiers with the same name as keywords are quoted.
                    pub async fn call_bool<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.bool)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                }
            }
        }
    }
}
