pub struct TheWorld {
    interface0: exports::foo::foo::simple::Guest,
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
            U: foo::foo::simple::Host,
        {
            foo::foo::simple::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::simple::Guest::new(
                &mut __exports
                    .instance("foo:foo/simple")
                    .ok_or_else(|| {
                        anyhow::anyhow!("exported instance `foo:foo/simple` not present")
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_simple(&self) -> &exports::foo::foo::simple::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod simple {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {
                fn f1(&mut self) -> ();
                fn f2(&mut self, a: u32) -> ();
                fn f3(&mut self, a: u32, b: u32) -> ();
                fn f4(&mut self) -> u32;
                fn f5(&mut self) -> (u32, u32);
                fn f6(&mut self, a: u32, b: u32, c: u32) -> (u32, u32, u32);
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
                let mut inst = linker.instance("foo:foo/simple")?;
                inst.func_wrap(
                    "f1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f1(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "f2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f2(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "f3",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0, arg1): (u32, u32)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f3(host, arg0, arg1);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "f4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f4(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "f5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f5(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "f6",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0, arg1, arg2): (u32, u32, u32)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f6(host, arg0, arg1, arg2);
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
                fn f1(&mut self) -> () {
                    Host::f1(*self)
                }
                fn f2(&mut self, a: u32) -> () {
                    Host::f2(*self, a)
                }
                fn f3(&mut self, a: u32, b: u32) -> () {
                    Host::f3(*self, a, b)
                }
                fn f4(&mut self) -> u32 {
                    Host::f4(*self)
                }
                fn f5(&mut self) -> (u32, u32) {
                    Host::f5(*self)
                }
                fn f6(&mut self, a: u32, b: u32, c: u32) -> (u32, u32, u32) {
                    Host::f6(*self, a, b, c)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod simple {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    f1: wasmtime::component::Func,
                    f2: wasmtime::component::Func,
                    f3: wasmtime::component::Func,
                    f4: wasmtime::component::Func,
                    f5: wasmtime::component::Func,
                    f6: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let f1 = *__exports.typed_func::<(), ()>("f1")?.func();
                        let f2 = *__exports.typed_func::<(u32,), ()>("f2")?.func();
                        let f3 = *__exports.typed_func::<(u32, u32), ()>("f3")?.func();
                        let f4 = *__exports.typed_func::<(), (u32,)>("f4")?.func();
                        let f5 = *__exports
                            .typed_func::<(), ((u32, u32),)>("f5")?
                            .func();
                        let f6 = *__exports
                            .typed_func::<(u32, u32, u32), ((u32, u32, u32),)>("f6")?
                            .func();
                        Ok(Guest { f1, f2, f3, f4, f5, f6 })
                    }
                    pub fn call_f1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.f1)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_f2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32,),
                                (),
                            >::new_unchecked(self.f2)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_f3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                        arg1: u32,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32, u32),
                                (),
                            >::new_unchecked(self.f3)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0, arg1))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_f4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.f4)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_f5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(u32, u32)> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((u32, u32),),
                            >::new_unchecked(self.f5)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_f6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                        arg1: u32,
                        arg2: u32,
                    ) -> wasmtime::Result<(u32, u32, u32)> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32, u32, u32),
                                ((u32, u32, u32),),
                            >::new_unchecked(self.f6)
                        };
                        let (ret0,) = callee
                            .call(store.as_context_mut(), (arg0, arg1, arg2))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
