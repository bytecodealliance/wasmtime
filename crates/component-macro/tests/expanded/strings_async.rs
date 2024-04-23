pub struct TheWorld {
    interface0: exports::foo::foo::strings::Guest,
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
            U: foo::foo::strings::Host + Send,
        {
            foo::foo::strings::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::strings::Guest::new(
                &mut __exports
                    .instance("foo:foo/strings")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/strings` not present"
                        )
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_strings(&self) -> &exports::foo::foo::strings::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod strings {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn a(&mut self, x: wasmtime::component::__internal::String) -> ();
                async fn b(&mut self) -> wasmtime::component::__internal::String;
                async fn c(
                    &mut self,
                    a: wasmtime::component::__internal::String,
                    b: wasmtime::component::__internal::String,
                ) -> wasmtime::component::__internal::String;
            }
            pub trait GetHost<T>: Send + Sync + Copy + 'static {
                fn get_host<'a>(&self, data: &'a mut T) -> impl Host;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl GetHost<T>,
            ) -> wasmtime::Result<()>
            where
                T: Send,
            {
                let mut inst = linker.instance("foo:foo/strings")?;
                inst.func_wrap_async(
                    "a",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::String,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::a(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "b",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::b(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "c",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                        ): (
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::String,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter.get_host(caller.data_mut());
                        let r = Host::c(host, arg0, arg1).await;
                        Ok((r,))
                    }),
                )?;
                Ok(())
            }
            impl<T, U, F> GetHost<T> for F
            where
                U: Host + Send,
                T: Send,
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
                U: Host + Send,
                T: Send,
            {
                add_to_linker_get_host(linker, get)
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                async fn a(&mut self, x: wasmtime::component::__internal::String) -> () {
                    Host::a(*self, x).await
                }
                async fn b(&mut self) -> wasmtime::component::__internal::String {
                    Host::b(*self).await
                }
                async fn c(
                    &mut self,
                    a: wasmtime::component::__internal::String,
                    b: wasmtime::component::__internal::String,
                ) -> wasmtime::component::__internal::String {
                    Host::c(*self, a, b).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod strings {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    a: wasmtime::component::Func,
                    b: wasmtime::component::Func,
                    c: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let a = *__exports.typed_func::<(&str,), ()>("a")?.func();
                        let b = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::String,),
                            >("b")?
                            .func();
                        let c = *__exports
                            .typed_func::<
                                (&str, &str),
                                (wasmtime::component::__internal::String,),
                            >("c")?
                            .func();
                        Ok(Guest { a, b, c })
                    }
                    pub async fn call_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &str,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&str,),
                                (),
                            >::new_unchecked(self.a)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_b<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::String>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::String,),
                            >::new_unchecked(self.b)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_c<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &str,
                        arg1: &str,
                    ) -> wasmtime::Result<wasmtime::component::__internal::String>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&str, &str),
                                (wasmtime::component::__internal::String,),
                            >::new_unchecked(self.c)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0, arg1))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
