pub struct TheWorld {
    interface0: exports::foo::foo::integers::Guest,
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
            U: foo::foo::integers::Host + Send,
        {
            foo::foo::integers::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::integers::Guest::new(
                &mut __exports
                    .instance("foo:foo/integers")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/integers` not present"
                        )
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_integers(&self) -> &exports::foo::foo::integers::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod integers {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn a1(&mut self, x: u8) -> ();
                async fn a2(&mut self, x: i8) -> ();
                async fn a3(&mut self, x: u16) -> ();
                async fn a4(&mut self, x: i16) -> ();
                async fn a5(&mut self, x: u32) -> ();
                async fn a6(&mut self, x: i32) -> ();
                async fn a7(&mut self, x: u64) -> ();
                async fn a8(&mut self, x: i64) -> ();
                async fn a9(
                    &mut self,
                    p1: u8,
                    p2: i8,
                    p3: u16,
                    p4: i16,
                    p5: u32,
                    p6: i32,
                    p7: u64,
                    p8: i64,
                ) -> ();
                async fn r1(&mut self) -> u8;
                async fn r2(&mut self) -> i8;
                async fn r3(&mut self) -> u16;
                async fn r4(&mut self) -> i16;
                async fn r5(&mut self) -> u32;
                async fn r6(&mut self) -> i32;
                async fn r7(&mut self) -> u64;
                async fn r8(&mut self) -> i64;
                async fn pair_ret(&mut self) -> (i64, u8);
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
                let mut inst = linker.instance("foo:foo/integers")?;
                inst.func_wrap_async(
                    "a1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u8,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a1(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i8,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a2(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u16,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a3(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i16,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a4(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a5(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a6",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i32,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a6(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a7",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u64,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a7(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a8",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i64,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a8(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "a9",
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
                        ): (u8, i8, u16, i16, u32, i32, u64, i64)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a9(
                                host,
                                arg0,
                                arg1,
                                arg2,
                                arg3,
                                arg4,
                                arg5,
                                arg6,
                                arg7,
                            )
                            .await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "r1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r1(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r2(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r3(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r4(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r5(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r6",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r6(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r7",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r7(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "r8",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r8(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "pair-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::pair_ret(host).await;
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
                async fn a1(&mut self, x: u8) -> () {
                    Host::a1(*self, x).await
                }
                async fn a2(&mut self, x: i8) -> () {
                    Host::a2(*self, x).await
                }
                async fn a3(&mut self, x: u16) -> () {
                    Host::a3(*self, x).await
                }
                async fn a4(&mut self, x: i16) -> () {
                    Host::a4(*self, x).await
                }
                async fn a5(&mut self, x: u32) -> () {
                    Host::a5(*self, x).await
                }
                async fn a6(&mut self, x: i32) -> () {
                    Host::a6(*self, x).await
                }
                async fn a7(&mut self, x: u64) -> () {
                    Host::a7(*self, x).await
                }
                async fn a8(&mut self, x: i64) -> () {
                    Host::a8(*self, x).await
                }
                async fn a9(
                    &mut self,
                    p1: u8,
                    p2: i8,
                    p3: u16,
                    p4: i16,
                    p5: u32,
                    p6: i32,
                    p7: u64,
                    p8: i64,
                ) -> () {
                    Host::a9(*self, p1, p2, p3, p4, p5, p6, p7, p8).await
                }
                async fn r1(&mut self) -> u8 {
                    Host::r1(*self).await
                }
                async fn r2(&mut self) -> i8 {
                    Host::r2(*self).await
                }
                async fn r3(&mut self) -> u16 {
                    Host::r3(*self).await
                }
                async fn r4(&mut self) -> i16 {
                    Host::r4(*self).await
                }
                async fn r5(&mut self) -> u32 {
                    Host::r5(*self).await
                }
                async fn r6(&mut self) -> i32 {
                    Host::r6(*self).await
                }
                async fn r7(&mut self) -> u64 {
                    Host::r7(*self).await
                }
                async fn r8(&mut self) -> i64 {
                    Host::r8(*self).await
                }
                async fn pair_ret(&mut self) -> (i64, u8) {
                    Host::pair_ret(*self).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod integers {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    a1: wasmtime::component::Func,
                    a2: wasmtime::component::Func,
                    a3: wasmtime::component::Func,
                    a4: wasmtime::component::Func,
                    a5: wasmtime::component::Func,
                    a6: wasmtime::component::Func,
                    a7: wasmtime::component::Func,
                    a8: wasmtime::component::Func,
                    a9: wasmtime::component::Func,
                    r1: wasmtime::component::Func,
                    r2: wasmtime::component::Func,
                    r3: wasmtime::component::Func,
                    r4: wasmtime::component::Func,
                    r5: wasmtime::component::Func,
                    r6: wasmtime::component::Func,
                    r7: wasmtime::component::Func,
                    r8: wasmtime::component::Func,
                    pair_ret: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let a1 = *__exports.typed_func::<(u8,), ()>("a1")?.func();
                        let a2 = *__exports.typed_func::<(i8,), ()>("a2")?.func();
                        let a3 = *__exports.typed_func::<(u16,), ()>("a3")?.func();
                        let a4 = *__exports.typed_func::<(i16,), ()>("a4")?.func();
                        let a5 = *__exports.typed_func::<(u32,), ()>("a5")?.func();
                        let a6 = *__exports.typed_func::<(i32,), ()>("a6")?.func();
                        let a7 = *__exports.typed_func::<(u64,), ()>("a7")?.func();
                        let a8 = *__exports.typed_func::<(i64,), ()>("a8")?.func();
                        let a9 = *__exports
                            .typed_func::<
                                (u8, i8, u16, i16, u32, i32, u64, i64),
                                (),
                            >("a9")?
                            .func();
                        let r1 = *__exports.typed_func::<(), (u8,)>("r1")?.func();
                        let r2 = *__exports.typed_func::<(), (i8,)>("r2")?.func();
                        let r3 = *__exports.typed_func::<(), (u16,)>("r3")?.func();
                        let r4 = *__exports.typed_func::<(), (i16,)>("r4")?.func();
                        let r5 = *__exports.typed_func::<(), (u32,)>("r5")?.func();
                        let r6 = *__exports.typed_func::<(), (i32,)>("r6")?.func();
                        let r7 = *__exports.typed_func::<(), (u64,)>("r7")?.func();
                        let r8 = *__exports.typed_func::<(), (i64,)>("r8")?.func();
                        let pair_ret = *__exports
                            .typed_func::<(), ((i64, u8),)>("pair-ret")?
                            .func();
                        Ok(Guest {
                            a1,
                            a2,
                            a3,
                            a4,
                            a5,
                            a6,
                            a7,
                            a8,
                            a9,
                            r1,
                            r2,
                            r3,
                            r4,
                            r5,
                            r6,
                            r7,
                            r8,
                            pair_ret,
                        })
                    }
                    pub async fn call_a1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u8,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u8,),
                                (),
                            >::new_unchecked(self.a1)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i8,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i8,),
                                (),
                            >::new_unchecked(self.a2)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u16,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u16,),
                                (),
                            >::new_unchecked(self.a3)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i16,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i16,),
                                (),
                            >::new_unchecked(self.a4)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32,),
                                (),
                            >::new_unchecked(self.a5)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i32,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i32,),
                                (),
                            >::new_unchecked(self.a6)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a7<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u64,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u64,),
                                (),
                            >::new_unchecked(self.a7)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i64,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i64,),
                                (),
                            >::new_unchecked(self.a8)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_a9<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u8,
                        arg1: i8,
                        arg2: u16,
                        arg3: i16,
                        arg4: u32,
                        arg5: i32,
                        arg6: u64,
                        arg7: i64,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u8, i8, u16, i16, u32, i32, u64, i64),
                                (),
                            >::new_unchecked(self.a9)
                        };
                        let () = callee
                            .call_async(
                                store.as_context_mut(),
                                (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7),
                            )
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_r1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u8>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u8,),
                            >::new_unchecked(self.r1)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i8>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i8,),
                            >::new_unchecked(self.r2)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u16>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u16,),
                            >::new_unchecked(self.r3)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i16>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i16,),
                            >::new_unchecked(self.r4)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.r5)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i32>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i32,),
                            >::new_unchecked(self.r6)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r7<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u64>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u64,),
                            >::new_unchecked(self.r7)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_r8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i64>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i64,),
                            >::new_unchecked(self.r8)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_pair_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(i64, u8)>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((i64, u8),),
                            >::new_unchecked(self.pair_ret)
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
