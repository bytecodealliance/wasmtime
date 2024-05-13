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
            U: foo::foo::integers::Host,
        {
            foo::foo::integers::add_to_linker(linker, get)?;
            Ok(())
        }
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate(&mut store, component)?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Instantiates a pre-instantiated module using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate_pre<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance_pre: &wasmtime::component::InstancePre<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = instance_pre.instantiate(&mut store)?;
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
            pub trait Host {
                fn a1(&mut self, x: u8) -> ();
                fn a2(&mut self, x: i8) -> ();
                fn a3(&mut self, x: u16) -> ();
                fn a4(&mut self, x: i16) -> ();
                fn a5(&mut self, x: u32) -> ();
                fn a6(&mut self, x: i32) -> ();
                fn a7(&mut self, x: u64) -> ();
                fn a8(&mut self, x: i64) -> ();
                fn a9(
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
                fn r1(&mut self) -> u8;
                fn r2(&mut self) -> i8;
                fn r3(&mut self) -> u16;
                fn r4(&mut self) -> i16;
                fn r5(&mut self) -> u32;
                fn r6(&mut self) -> i32;
                fn r7(&mut self) -> u64;
                fn r8(&mut self) -> i64;
                fn pair_ret(&mut self) -> (i64, u8);
            }
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Host + Send + Sync + Copy + 'static {
                type Host: Host;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("foo:foo/integers")?;
                inst.func_wrap(
                    "a1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u8,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a1(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i8,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a2(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u16,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a3(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i16,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a4(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a5(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a6",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i32,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a6(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a7",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u64,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a7(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a8",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i64,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a8(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
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
                    {
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
                        );
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "r1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r1(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r2(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r3(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r4(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r5(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r6",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r6(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r7",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r7(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "r8",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::r8(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "pair-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::pair_ret(host);
                        Ok((r,))
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {
                fn a1(&mut self, x: u8) -> () {
                    Host::a1(*self, x)
                }
                fn a2(&mut self, x: i8) -> () {
                    Host::a2(*self, x)
                }
                fn a3(&mut self, x: u16) -> () {
                    Host::a3(*self, x)
                }
                fn a4(&mut self, x: i16) -> () {
                    Host::a4(*self, x)
                }
                fn a5(&mut self, x: u32) -> () {
                    Host::a5(*self, x)
                }
                fn a6(&mut self, x: i32) -> () {
                    Host::a6(*self, x)
                }
                fn a7(&mut self, x: u64) -> () {
                    Host::a7(*self, x)
                }
                fn a8(&mut self, x: i64) -> () {
                    Host::a8(*self, x)
                }
                fn a9(
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
                    Host::a9(*self, p1, p2, p3, p4, p5, p6, p7, p8)
                }
                fn r1(&mut self) -> u8 {
                    Host::r1(*self)
                }
                fn r2(&mut self) -> i8 {
                    Host::r2(*self)
                }
                fn r3(&mut self) -> u16 {
                    Host::r3(*self)
                }
                fn r4(&mut self) -> i16 {
                    Host::r4(*self)
                }
                fn r5(&mut self) -> u32 {
                    Host::r5(*self)
                }
                fn r6(&mut self) -> i32 {
                    Host::r6(*self)
                }
                fn r7(&mut self) -> u64 {
                    Host::r7(*self)
                }
                fn r8(&mut self) -> i64 {
                    Host::r8(*self)
                }
                fn pair_ret(&mut self) -> (i64, u8) {
                    Host::pair_ret(*self)
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
                    pub fn call_a1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u8,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u8,),
                                (),
                            >::new_unchecked(self.a1)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i8,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i8,),
                                (),
                            >::new_unchecked(self.a2)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u16,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u16,),
                                (),
                            >::new_unchecked(self.a3)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i16,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i16,),
                                (),
                            >::new_unchecked(self.a4)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32,),
                                (),
                            >::new_unchecked(self.a5)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i32,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i32,),
                                (),
                            >::new_unchecked(self.a6)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a7<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u64,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u64,),
                                (),
                            >::new_unchecked(self.a7)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i64,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i64,),
                                (),
                            >::new_unchecked(self.a8)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a9<S: wasmtime::AsContextMut>(
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
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u8, i8, u16, i16, u32, i32, u64, i64),
                                (),
                            >::new_unchecked(self.a9)
                        };
                        let () = callee
                            .call(
                                store.as_context_mut(),
                                (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7),
                            )?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_r1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u8> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u8,),
                            >::new_unchecked(self.r1)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i8> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i8,),
                            >::new_unchecked(self.r2)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u16> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u16,),
                            >::new_unchecked(self.r3)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i16> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i16,),
                            >::new_unchecked(self.r4)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.r5)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i32,),
                            >::new_unchecked(self.r6)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r7<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u64> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u64,),
                            >::new_unchecked(self.r7)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_r8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i64> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i64,),
                            >::new_unchecked(self.r8)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_pair_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(i64, u8)> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((i64, u8),),
                            >::new_unchecked(self.pair_ret)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
