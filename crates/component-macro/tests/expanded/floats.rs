pub struct TheWorld {
    interface0: exports::foo::foo::floats::Guest,
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
            U: foo::foo::floats::Host,
        {
            foo::foo::floats::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::floats::Guest::new(
                &mut __exports
                    .instance("foo:foo/floats")
                    .ok_or_else(|| {
                        anyhow::anyhow!("exported instance `foo:foo/floats` not present")
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_floats(&self) -> &exports::foo::foo::floats::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod floats {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {
                fn float32_param(&mut self, x: f32) -> ();
                fn float64_param(&mut self, x: f64) -> ();
                fn float32_result(&mut self) -> f32;
                fn float64_result(&mut self) -> f64;
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
                let mut inst = linker.instance("foo:foo/floats")?;
                inst.func_wrap(
                    "float32-param",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (f32,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::float32_param(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "float64-param",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (f64,)| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::float64_param(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "float32-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::float32_result(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "float64-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::float64_result(host);
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
                fn float32_param(&mut self, x: f32) -> () {
                    Host::float32_param(*self, x)
                }
                fn float64_param(&mut self, x: f64) -> () {
                    Host::float64_param(*self, x)
                }
                fn float32_result(&mut self) -> f32 {
                    Host::float32_result(*self)
                }
                fn float64_result(&mut self) -> f64 {
                    Host::float64_result(*self)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod floats {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    float32_param: wasmtime::component::Func,
                    float64_param: wasmtime::component::Func,
                    float32_result: wasmtime::component::Func,
                    float64_result: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let float32_param = *__exports
                            .typed_func::<(f32,), ()>("float32-param")?
                            .func();
                        let float64_param = *__exports
                            .typed_func::<(f64,), ()>("float64-param")?
                            .func();
                        let float32_result = *__exports
                            .typed_func::<(), (f32,)>("float32-result")?
                            .func();
                        let float64_result = *__exports
                            .typed_func::<(), (f64,)>("float64-result")?
                            .func();
                        Ok(Guest {
                            float32_param,
                            float64_param,
                            float32_result,
                            float64_result,
                        })
                    }
                    pub fn call_float32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: f32,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (f32,),
                                (),
                            >::new_unchecked(self.float32_param)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_float64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: f64,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (f64,),
                                (),
                            >::new_unchecked(self.float64_param)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_float32_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<f32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (f32,),
                            >::new_unchecked(self.float32_result)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_float64_result<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<f64> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (f64,),
                            >::new_unchecked(self.float64_result)
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
