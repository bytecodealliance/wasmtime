/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `w`.
///
/// This structure is created through [`WPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct WPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::foo::foo::simple_export::GuestPre,
    interface1: exports::foo::foo::export_using_import::GuestPre,
    interface2: exports::foo::foo::export_using_export1::GuestPre,
    interface3: exports::foo::foo::export_using_export2::GuestPre,
}
impl<T> Clone for WPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            interface0: self.interface0.clone(),
            interface1: self.interface1.clone(),
            interface2: self.interface2.clone(),
            interface3: self.interface3.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `w`.
///
/// This structure is created through either
/// [`W::instantiate_async`] or by first creating
/// a [`WPre`] followed by using
/// [`WPre::instantiate_async`].
pub struct W {
    interface0: exports::foo::foo::simple_export::Guest,
    interface1: exports::foo::foo::export_using_import::Guest,
    interface2: exports::foo::foo::export_using_export1::Guest,
    interface3: exports::foo::foo::export_using_export2::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> WPre<_T> {
        /// Creates a new copy of `WPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::foo::foo::simple_export::GuestPre::new(
                _component,
            )?;
            let interface1 = exports::foo::foo::export_using_import::GuestPre::new(
                _component,
            )?;
            let interface2 = exports::foo::foo::export_using_export1::GuestPre::new(
                _component,
            )?;
            let interface3 = exports::foo::foo::export_using_export2::GuestPre::new(
                _component,
            )?;
            Ok(WPre {
                instance_pre,
                interface0,
                interface1,
                interface2,
                interface3,
            })
        }
        /// Instantiates a new instance of [`W`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<W>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            let interface1 = self.interface1.load(&mut store, &_instance)?;
            let interface2 = self.interface2.load(&mut store, &_instance)?;
            let interface3 = self.interface3.load(&mut store, &_instance)?;
            Ok(W {
                interface0,
                interface1,
                interface2,
                interface3,
            })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl W {
        /// Convenience wrapper around [`WPre::new`] and
        /// [`WPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<W>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            WPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::transitive_import::Host + Send,
        {
            foo::foo::transitive_import::add_to_linker(linker, get)?;
            Ok(())
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
            #[wasmtime::component::__internal::async_trait]
            pub trait HostY {
                fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Y>,
                ) -> wasmtime::Result<()>;
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: HostY + ?Sized + Send> HostY for &mut _T {
                fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Y>,
                ) -> wasmtime::Result<()> {
                    HostY::drop(*self, rep)
                }
            }
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send + HostY {}
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
                let mut inst = linker.instance("foo:foo/transitive-import")?;
                inst.resource(
                    "y",
                    wasmtime::component::ResourceType::host::<Y>(),
                    move |mut store, rep| -> wasmtime::Result<()> {
                        HostY::drop(
                            &mut host_getter(store.data_mut()),
                            wasmtime::component::Resource::new_own(rep),
                        )
                    },
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
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
                #[derive(Clone)]
                pub struct GuestPre {
                    constructor_a_constructor: wasmtime::component::ComponentExportIndex,
                    static_a_static_a: wasmtime::component::ComponentExportIndex,
                    method_a_method_a: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/simple-export")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/simple-export`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/simple-export` does \
                  not have export `{name}`"
                                    )
                                })
                        };
                        let constructor_a_constructor = _lookup("[constructor]a")?;
                        let static_a_static_a = _lookup("[static]a.static-a")?;
                        let method_a_method_a = _lookup("[method]a.method-a")?;
                        Ok(GuestPre {
                            constructor_a_constructor,
                            static_a_static_a,
                            method_a_method_a,
                        })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let constructor_a_constructor = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >(&mut store, &self.constructor_a_constructor)?
                            .func();
                        let static_a_static_a = *_instance
                            .get_typed_func::<
                                (),
                                (u32,),
                            >(&mut store, &self.static_a_static_a)?
                            .func();
                        let method_a_method_a = *_instance
                            .get_typed_func::<
                                (wasmtime::component::ResourceAny,),
                                (u32,),
                            >(&mut store, &self.method_a_method_a)?
                            .func();
                        Ok(Guest {
                            constructor_a_constructor,
                            static_a_static_a,
                            method_a_method_a,
                        })
                    }
                }
                impl Guest {
                    pub fn a(&self) -> GuestA<'_> {
                        GuestA { funcs: self }
                    }
                }
                impl GuestA<'_> {
                    pub async fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_a_constructor)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_static_a<S: wasmtime::AsContextMut>(
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
                            >::new_unchecked(self.funcs.static_a_static_a)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_method_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::ResourceAny,
                    ) -> wasmtime::Result<u32>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::ResourceAny,),
                                (u32,),
                            >::new_unchecked(self.funcs.method_a_method_a)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
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
                #[derive(Clone)]
                pub struct GuestPre {
                    constructor_a_constructor: wasmtime::component::ComponentExportIndex,
                    static_a_static_a: wasmtime::component::ComponentExportIndex,
                    method_a_method_a: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/export-using-import")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/export-using-import`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/export-using-import` does \
                  not have export `{name}`"
                                    )
                                })
                        };
                        let constructor_a_constructor = _lookup("[constructor]a")?;
                        let static_a_static_a = _lookup("[static]a.static-a")?;
                        let method_a_method_a = _lookup("[method]a.method-a")?;
                        Ok(GuestPre {
                            constructor_a_constructor,
                            static_a_static_a,
                            method_a_method_a,
                        })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let constructor_a_constructor = *_instance
                            .get_typed_func::<
                                (wasmtime::component::Resource<Y>,),
                                (wasmtime::component::ResourceAny,),
                            >(&mut store, &self.constructor_a_constructor)?
                            .func();
                        let static_a_static_a = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::Resource<Y>,),
                            >(&mut store, &self.static_a_static_a)?
                            .func();
                        let method_a_method_a = *_instance
                            .get_typed_func::<
                                (
                                    wasmtime::component::ResourceAny,
                                    wasmtime::component::Resource<Y>,
                                ),
                                (wasmtime::component::Resource<Y>,),
                            >(&mut store, &self.method_a_method_a)?
                            .func();
                        Ok(Guest {
                            constructor_a_constructor,
                            static_a_static_a,
                            method_a_method_a,
                        })
                    }
                }
                impl Guest {
                    pub fn a(&self) -> GuestA<'_> {
                        GuestA { funcs: self }
                    }
                }
                impl GuestA<'_> {
                    pub async fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::Resource<Y>,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::Resource<Y>,),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_a_constructor)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_static_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Resource<Y>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::Resource<Y>,),
                            >::new_unchecked(self.funcs.static_a_static_a)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_method_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::ResourceAny,
                        arg1: wasmtime::component::Resource<Y>,
                    ) -> wasmtime::Result<wasmtime::component::Resource<Y>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::ResourceAny,
                                    wasmtime::component::Resource<Y>,
                                ),
                                (wasmtime::component::Resource<Y>,),
                            >::new_unchecked(self.funcs.method_a_method_a)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0, arg1))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
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
                #[derive(Clone)]
                pub struct GuestPre {
                    constructor_a_constructor: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/export-using-export1")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/export-using-export1`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/export-using-export1` does \
                  not have export `{name}`"
                                    )
                                })
                        };
                        let constructor_a_constructor = _lookup("[constructor]a")?;
                        Ok(GuestPre {
                            constructor_a_constructor,
                        })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let constructor_a_constructor = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >(&mut store, &self.constructor_a_constructor)?
                            .func();
                        Ok(Guest { constructor_a_constructor })
                    }
                }
                impl Guest {
                    pub fn a(&self) -> GuestA<'_> {
                        GuestA { funcs: self }
                    }
                }
                impl GuestA<'_> {
                    pub async fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_a_constructor)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
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
                #[derive(Clone)]
                pub struct GuestPre {
                    constructor_b_constructor: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/export-using-export2")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/export-using-export2`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/export-using-export2` does \
                  not have export `{name}`"
                                    )
                                })
                        };
                        let constructor_b_constructor = _lookup("[constructor]b")?;
                        Ok(GuestPre {
                            constructor_b_constructor,
                        })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let constructor_b_constructor = *_instance
                            .get_typed_func::<
                                (wasmtime::component::ResourceAny,),
                                (wasmtime::component::ResourceAny,),
                            >(&mut store, &self.constructor_b_constructor)?
                            .func();
                        Ok(Guest { constructor_b_constructor })
                    }
                }
                impl Guest {
                    pub fn b(&self) -> GuestB<'_> {
                        GuestB { funcs: self }
                    }
                }
                impl GuestB<'_> {
                    pub async fn call_constructor<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::ResourceAny,
                    ) -> wasmtime::Result<wasmtime::component::ResourceAny>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::ResourceAny,),
                                (wasmtime::component::ResourceAny,),
                            >::new_unchecked(self.funcs.constructor_b_constructor)
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
