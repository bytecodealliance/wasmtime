/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::foo::foo::simple::GuestPre,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure is created through either
/// [`TheWorld::instantiate_async`] or by first creating
/// a [`TheWorldPre`] followed by using
/// [`TheWorldPre::instantiate_async`].
pub struct TheWorld {
    interface0: exports::foo::foo::simple::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> TheWorldPre<_T> {
        /// Creates a new copy of `TheWorldPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::foo::foo::simple::GuestPre::new(_component)?;
            Ok(TheWorldPre {
                instance_pre,
                interface0,
            })
        }
        /// Instantiates a new instance of [`TheWorld`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<TheWorld>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheWorld { interface0 })
        }
    }
    impl TheWorld {
        /// Convenience wrapper around [`TheWorldPre::new`] and
        /// [`TheWorldPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheWorld>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::simple::Host + Send,
        {
            foo::foo::simple::add_to_linker(linker, get)?;
            Ok(())
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
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn f1(&mut self) -> ();
                async fn f2(&mut self, a: u32) -> ();
                async fn f3(&mut self, a: u32, b: u32) -> ();
                async fn f4(&mut self) -> u32;
                async fn f5(&mut self) -> (u32, u32);
                async fn f6(&mut self, a: u32, b: u32, c: u32) -> (u32, u32, u32);
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
                let mut inst = linker.instance("foo:foo/simple")?;
                inst.func_wrap_async(
                    "f1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f1(host).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "f2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f2(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "f3",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0, arg1): (u32, u32)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f3(host, arg0, arg1).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "f4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f4(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "f5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f5(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "f6",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0, arg1, arg2): (u32, u32, u32)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f6(host, arg0, arg1, arg2).await;
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
                async fn f1(&mut self) -> () {
                    Host::f1(*self).await
                }
                async fn f2(&mut self, a: u32) -> () {
                    Host::f2(*self, a).await
                }
                async fn f3(&mut self, a: u32, b: u32) -> () {
                    Host::f3(*self, a, b).await
                }
                async fn f4(&mut self) -> u32 {
                    Host::f4(*self).await
                }
                async fn f5(&mut self) -> (u32, u32) {
                    Host::f5(*self).await
                }
                async fn f6(&mut self, a: u32, b: u32, c: u32) -> (u32, u32, u32) {
                    Host::f6(*self, a, b, c).await
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
                pub struct GuestPre {
                    f1: wasmtime::component::ComponentExportIndex,
                    f2: wasmtime::component::ComponentExportIndex,
                    f3: wasmtime::component::ComponentExportIndex,
                    f4: wasmtime::component::ComponentExportIndex,
                    f5: wasmtime::component::ComponentExportIndex,
                    f6: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/simple")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/simple`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/simple` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let f1 = _lookup("f1")?;
                        let f2 = _lookup("f2")?;
                        let f3 = _lookup("f3")?;
                        let f4 = _lookup("f4")?;
                        let f5 = _lookup("f5")?;
                        let f6 = _lookup("f6")?;
                        Ok(GuestPre { f1, f2, f3, f4, f5, f6 })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let f1 = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.f1)?
                            .func();
                        let f2 = *_instance
                            .get_typed_func::<(u32,), ()>(&mut store, &self.f2)?
                            .func();
                        let f3 = *_instance
                            .get_typed_func::<(u32, u32), ()>(&mut store, &self.f3)?
                            .func();
                        let f4 = *_instance
                            .get_typed_func::<(), (u32,)>(&mut store, &self.f4)?
                            .func();
                        let f5 = *_instance
                            .get_typed_func::<(), ((u32, u32),)>(&mut store, &self.f5)?
                            .func();
                        let f6 = *_instance
                            .get_typed_func::<
                                (u32, u32, u32),
                                ((u32, u32, u32),),
                            >(&mut store, &self.f6)?
                            .func();
                        Ok(Guest { f1, f2, f3, f4, f5, f6 })
                    }
                }
                impl Guest {
                    pub async fn call_f1<S: wasmtime::AsContextMut>(
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
                            >::new_unchecked(self.f1)
                        };
                        let () = callee.call_async(store.as_context_mut(), ()).await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_f2<S: wasmtime::AsContextMut>(
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
                            >::new_unchecked(self.f2)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_f3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                        arg1: u32,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32, u32),
                                (),
                            >::new_unchecked(self.f3)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0, arg1))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_f4<S: wasmtime::AsContextMut>(
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
                            >::new_unchecked(self.f4)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_f5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(u32, u32)>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((u32, u32),),
                            >::new_unchecked(self.f5)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_f6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                        arg1: u32,
                        arg2: u32,
                    ) -> wasmtime::Result<(u32, u32, u32)>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32, u32, u32),
                                ((u32, u32, u32),),
                            >::new_unchecked(self.f6)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0, arg1, arg2))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
