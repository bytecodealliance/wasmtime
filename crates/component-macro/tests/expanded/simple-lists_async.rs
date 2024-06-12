/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `my-world`.
///
/// This structure is created through [`MyWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct MyWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::foo::foo::simple_lists::GuestPre,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `my-world`.
///
/// This structure is created through either
/// [`MyWorld::instantiate_async`] or by first creating
/// a [`MyWorldPre`] followed by using
/// [`MyWorldPre::instantiate_async`].
pub struct MyWorld {
    interface0: exports::foo::foo::simple_lists::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> MyWorldPre<_T> {
        /// Creates a new copy of `MyWorldPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::foo::foo::simple_lists::GuestPre::new(_component)?;
            Ok(MyWorldPre {
                instance_pre,
                interface0,
            })
        }
        /// Instantiates a new instance of [`MyWorld`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub async fn instantiate_async(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<MyWorld>
        where
            _T: Send,
        {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(MyWorld { interface0 })
        }
    }
    impl MyWorld {
        /// Convenience wrapper around [`MyWorldPre::new`] and
        /// [`MyWorldPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<MyWorld>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            MyWorldPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::simple_lists::Host + Send,
        {
            foo::foo::simple_lists::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_simple_lists(&self) -> &exports::foo::foo::simple_lists::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod simple_lists {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn simple_list1(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<u32>,
                ) -> ();
                async fn simple_list2(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u32>;
                async fn simple_list3(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<u32>,
                    b: wasmtime::component::__internal::Vec<u32>,
                ) -> (
                    wasmtime::component::__internal::Vec<u32>,
                    wasmtime::component::__internal::Vec<u32>,
                );
                async fn simple_list4(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::Vec<u32>,
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::Vec<u32>,
                >;
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
                let mut inst = linker.instance("foo:foo/simple-lists")?;
                inst.func_wrap_async(
                    "simple-list1",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u32>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list1(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "simple-list2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list2(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "simple-list3",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                        ): (
                            wasmtime::component::__internal::Vec<u32>,
                            wasmtime::component::__internal::Vec<u32>,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list3(host, arg0, arg1).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "simple-list4",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::Vec<u32>,
                            >,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list4(host, arg0).await;
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
                async fn simple_list1(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<u32>,
                ) -> () {
                    Host::simple_list1(*self, l).await
                }
                async fn simple_list2(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u32> {
                    Host::simple_list2(*self).await
                }
                async fn simple_list3(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<u32>,
                    b: wasmtime::component::__internal::Vec<u32>,
                ) -> (
                    wasmtime::component::__internal::Vec<u32>,
                    wasmtime::component::__internal::Vec<u32>,
                ) {
                    Host::simple_list3(*self, a, b).await
                }
                async fn simple_list4(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::Vec<u32>,
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::Vec<u32>,
                > {
                    Host::simple_list4(*self, l).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod simple_lists {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    simple_list1: wasmtime::component::Func,
                    simple_list2: wasmtime::component::Func,
                    simple_list3: wasmtime::component::Func,
                    simple_list4: wasmtime::component::Func,
                }
                pub struct GuestPre {
                    simple_list1: wasmtime::component::ComponentExportIndex,
                    simple_list2: wasmtime::component::ComponentExportIndex,
                    simple_list3: wasmtime::component::ComponentExportIndex,
                    simple_list4: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/simple-lists")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/simple-lists`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/simple-lists` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let simple_list1 = _lookup("simple-list1")?;
                        let simple_list2 = _lookup("simple-list2")?;
                        let simple_list3 = _lookup("simple-list3")?;
                        let simple_list4 = _lookup("simple-list4")?;
                        Ok(GuestPre {
                            simple_list1,
                            simple_list2,
                            simple_list3,
                            simple_list4,
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
                        let simple_list1 = *_instance
                            .get_typed_func::<
                                (&[u32],),
                                (),
                            >(&mut store, &self.simple_list1)?
                            .func();
                        let simple_list2 = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >(&mut store, &self.simple_list2)?
                            .func();
                        let simple_list3 = *_instance
                            .get_typed_func::<
                                (&[u32], &[u32]),
                                (
                                    (
                                        wasmtime::component::__internal::Vec<u32>,
                                        wasmtime::component::__internal::Vec<u32>,
                                    ),
                                ),
                            >(&mut store, &self.simple_list3)?
                            .func();
                        let simple_list4 = *_instance
                            .get_typed_func::<
                                (&[wasmtime::component::__internal::Vec<u32>],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::Vec<u32>,
                                    >,
                                ),
                            >(&mut store, &self.simple_list4)?
                            .func();
                        Ok(Guest {
                            simple_list1,
                            simple_list2,
                            simple_list3,
                            simple_list4,
                        })
                    }
                }
                impl Guest {
                    pub async fn call_simple_list1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u32],),
                                (),
                            >::new_unchecked(self.simple_list1)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_simple_list2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >::new_unchecked(self.simple_list2)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_simple_list3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u32],
                        arg1: &[u32],
                    ) -> wasmtime::Result<
                        (
                            wasmtime::component::__internal::Vec<u32>,
                            wasmtime::component::__internal::Vec<u32>,
                        ),
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u32], &[u32]),
                                (
                                    (
                                        wasmtime::component::__internal::Vec<u32>,
                                        wasmtime::component::__internal::Vec<u32>,
                                    ),
                                ),
                            >::new_unchecked(self.simple_list3)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0, arg1))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_simple_list4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[wasmtime::component::__internal::Vec<u32>],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::Vec<u32>,
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[wasmtime::component::__internal::Vec<u32>],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::Vec<u32>,
                                    >,
                                ),
                            >::new_unchecked(self.simple_list4)
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
