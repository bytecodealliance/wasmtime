pub struct W {
    interface0: exports::foo::foo::simple_export::Guest,
    interface1: exports::foo::foo::export_using_import::Guest,
    interface2: exports::foo::foo::export_using_export1::Guest,
    interface3: exports::foo::foo::export_using_export2::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl W {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::transitive_import::Host,
        {
            foo::foo::transitive_import::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::simple_export::Guest::new(
                &mut __exports
                    .instance("foo:foo/simple-export")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/simple-export` not present"
                        )
                    })?,
            )?;
            let interface1 = exports::foo::foo::export_using_import::Guest::new(
                &mut __exports
                    .instance("foo:foo/export-using-import")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/export-using-import` not present"
                        )
                    })?,
            )?;
            let interface2 = exports::foo::foo::export_using_export1::Guest::new(
                &mut __exports
                    .instance("foo:foo/export-using-export1")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/export-using-export1` not present"
                        )
                    })?,
            )?;
            let interface3 = exports::foo::foo::export_using_export2::Guest::new(
                &mut __exports
                    .instance("foo:foo/export-using-export2")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/export-using-export2` not present"
                        )
                    })?,
            )?;
            Ok(W {
                interface0,
                interface1,
                interface2,
                interface3,
            })
        }
        pub fn foo_foo_simple_export(&self) -> &exports::foo::foo::simple_export::Guest {
            &self.interface0
        }
        pub fn foo_foo_export_using_import(
            &self,
        ) -> &exports::foo::foo::export_using_import::Guest {
            &self.interface1
        }
        pub fn foo_foo_export_using_export1(
            &self,
        ) -> &exports::foo::foo::export_using_export1::Guest {
            &self.interface2
        }
        pub fn foo_foo_export_using_export2(
            &self,
        ) -> &exports::foo::foo::export_using_export2::Guest {
            &self.interface3
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod transitive_import {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub enum Y {}
            pub trait HostY {
                fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Y>,
                ) -> wasmtime::Result<()>;
            }
            impl<_T: HostY + ?Sized> HostY for &mut _T {
                fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Y>,
                ) -> wasmtime::Result<()> {
                    HostY::drop(*self, rep)
                }
            }
            pub trait Host: HostY {}
            pub trait GetHost<T>: Send + Sync + Copy + 'static {
                fn get_host<'a>(&self, data: &'a mut T) -> impl Host;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl GetHost<T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("foo:foo/transitive-import")?;
                inst.resource(
                    "y",
                    wasmtime::component::ResourceType::host::<Y>(),
                    move |mut store, rep| -> wasmtime::Result<()> {
                        HostY::drop(
                            &mut host_getter.get_host(store.data_mut()),
                            wasmtime::component::Resource::new_own(rep),
                        )
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
            impl<_T: Host + ?Sized> Host for &mut _T {}
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod simple_export {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type A = wasmtime::component::ResourceAny;
                pub struct GuestA<'a> {
                    funcs: &'a Guest,
                }
                pub struct Guest {
                    constructor_a_constructor: wasmtime::component::Func,
                    static_a_static_a: wasmtime::component::Func,
                    method_a_method_a: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let constructor_a_constructor = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >("[constructor]a")?
                            .func();
                        let static_a_static_a = *__exports
                            .typed_func::<(), (u32,)>("[static]a.static-a")?
                            .func();
                        let method_a_method_a = *__exports
                            .typed_func::<
                                (wasmtime::component::ResourceAny,),
                                (u32,),
                            >("[method]a.method-a")?
                            .func();
                        Ok(Guest {
                            constructor_a_constructor,
                            static_a_static_a,
                            method_a_method_a,
                        })
                    }
                    pub fn a(&self) -> GuestA<'_> {
                        GuestA { funcs: self }
                    }
                }
                impl GuestA<'_> {
                    pub fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_a_constructor)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_static_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.funcs.static_a_static_a)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_method_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::ResourceAny,
                    ) -> wasmtime::Result<u32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::ResourceAny,),
                                (u32,),
                            >::new_unchecked(self.funcs.method_a_method_a)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
            #[allow(clippy::all)]
            pub mod export_using_import {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type Y = super::super::super::super::foo::foo::transitive_import::Y;
                pub type A = wasmtime::component::ResourceAny;
                pub struct GuestA<'a> {
                    funcs: &'a Guest,
                }
                pub struct Guest {
                    constructor_a_constructor: wasmtime::component::Func,
                    static_a_static_a: wasmtime::component::Func,
                    method_a_method_a: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let constructor_a_constructor = *__exports
                            .typed_func::<
                                (wasmtime::component::Resource<Y>,),
                                (wasmtime::component::ResourceAny,),
                            >("[constructor]a")?
                            .func();
                        let static_a_static_a = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::Resource<Y>,),
                            >("[static]a.static-a")?
                            .func();
                        let method_a_method_a = *__exports
                            .typed_func::<
                                (
                                    wasmtime::component::ResourceAny,
                                    wasmtime::component::Resource<Y>,
                                ),
                                (wasmtime::component::Resource<Y>,),
                            >("[method]a.method-a")?
                            .func();
                        Ok(Guest {
                            constructor_a_constructor,
                            static_a_static_a,
                            method_a_method_a,
                        })
                    }
                    pub fn a(&self) -> GuestA<'_> {
                        GuestA { funcs: self }
                    }
                }
                impl GuestA<'_> {
                    pub fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::Resource<Y>,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::Resource<Y>,),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_a_constructor)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_static_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Resource<Y>> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::Resource<Y>,),
                            >::new_unchecked(self.funcs.static_a_static_a)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_method_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::ResourceAny,
                        arg1: wasmtime::component::Resource<Y>,
                    ) -> wasmtime::Result<wasmtime::component::Resource<Y>> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::ResourceAny,
                                    wasmtime::component::Resource<Y>,
                                ),
                                (wasmtime::component::Resource<Y>,),
                            >::new_unchecked(self.funcs.method_a_method_a)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0, arg1))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
            #[allow(clippy::all)]
            pub mod export_using_export1 {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type A = wasmtime::component::ResourceAny;
                pub struct GuestA<'a> {
                    funcs: &'a Guest,
                }
                pub struct Guest {
                    constructor_a_constructor: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let constructor_a_constructor = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >("[constructor]a")?
                            .func();
                        Ok(Guest { constructor_a_constructor })
                    }
                    pub fn a(&self) -> GuestA<'_> {
                        GuestA { funcs: self }
                    }
                }
                impl GuestA<'_> {
                    pub fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_a_constructor)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
            #[allow(clippy::all)]
            pub mod export_using_export2 {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type A = super::super::super::super::exports::foo::foo::export_using_export1::A;
                pub type B = wasmtime::component::ResourceAny;
                pub struct GuestB<'a> {
                    funcs: &'a Guest,
                }
                pub struct Guest {
                    constructor_b_constructor: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let constructor_b_constructor = *__exports
                            .typed_func::<
                                (wasmtime::component::ResourceAny,),
                                (wasmtime::component::ResourceAny,),
                            >("[constructor]b")?
                            .func();
                        Ok(Guest { constructor_b_constructor })
                    }
                    pub fn b(&self) -> GuestB<'_> {
                        GuestB { funcs: self }
                    }
                }
                impl GuestB<'_> {
                    pub fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::ResourceAny,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::ResourceAny,),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_b_constructor)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
