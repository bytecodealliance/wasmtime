pub struct TheWorld {
    interface0: exports::foo::foo::multi_return::Guest,
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
            U: foo::foo::multi_return::Host,
        {
            foo::foo::multi_return::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::multi_return::Guest::new(
                &mut __exports
                    .instance("foo:foo/multi-return")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/multi-return` not present"
                        )
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_multi_return(&self) -> &exports::foo::foo::multi_return::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod multi_return {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {
                fn mra(&mut self) -> ();
                fn mrb(&mut self) -> ();
                fn mrc(&mut self) -> u32;
                fn mrd(&mut self) -> u32;
                fn mre(&mut self) -> (u32, f32);
            }
            pub trait GetHost<T>: Send + Sync + Copy + 'static {
                fn get_host<'a>(&self, data: &'a mut T) -> impl Host;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl GetHost<T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("foo:foo/multi-return")?;
                inst.func_wrap(
                    "mra",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::mra(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "mrb",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::mrb(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "mrc",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::mrc(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "mrd",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::mrd(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "mre",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::mre(host);
                        Ok(r)
                    },
                )?;
                Ok(())
            }
            impl<T, U, F> GetHost<T> for F
            where
                U: Host,
                F: Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            {
                fn get_host<'a>(&self, data: &'a mut T) -> impl Host {
                    self(data)
                }
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
                fn mra(&mut self) -> () {
                    Host::mra(*self)
                }
                fn mrb(&mut self) -> () {
                    Host::mrb(*self)
                }
                fn mrc(&mut self) -> u32 {
                    Host::mrc(*self)
                }
                fn mrd(&mut self) -> u32 {
                    Host::mrd(*self)
                }
                fn mre(&mut self) -> (u32, f32) {
                    Host::mre(*self)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod multi_return {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    mra: wasmtime::component::Func,
                    mrb: wasmtime::component::Func,
                    mrc: wasmtime::component::Func,
                    mrd: wasmtime::component::Func,
                    mre: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let mra = *__exports.typed_func::<(), ()>("mra")?.func();
                        let mrb = *__exports.typed_func::<(), ()>("mrb")?.func();
                        let mrc = *__exports.typed_func::<(), (u32,)>("mrc")?.func();
                        let mrd = *__exports.typed_func::<(), (u32,)>("mrd")?.func();
                        let mre = *__exports.typed_func::<(), (u32, f32)>("mre")?.func();
                        Ok(Guest { mra, mrb, mrc, mrd, mre })
                    }
                    pub fn call_mra<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.mra)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_mrb<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.mrb)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_mrc<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.mrc)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_mrd<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.mrd)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_mre<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(u32, f32)> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32, f32),
                            >::new_unchecked(self.mre)
                        };
                        let (ret0, ret1) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok((ret0, ret1))
                    }
                }
            }
        }
    }
}
