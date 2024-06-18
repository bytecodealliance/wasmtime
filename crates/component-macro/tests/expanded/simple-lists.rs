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
/// [`MyWorld::instantiate`] or by first creating
/// a [`MyWorldPre`] followed by using
/// [`MyWorldPre::instantiate`].
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
        pub fn instantiate(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<MyWorld> {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate(&mut store)?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(MyWorld { interface0 })
        }
    }
    impl MyWorld {
        /// Convenience wrapper around [`MyWorldPre::new`] and
        /// [`MyWorldPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<MyWorld> {
            let pre = linker.instantiate_pre(component)?;
            MyWorldPre::new(pre)?.instantiate(store)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::simple_lists::Host,
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
            pub trait Host {
                fn simple_list1(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<u32>,
                ) -> ();
                fn simple_list2(&mut self) -> wasmtime::component::__internal::Vec<u32>;
                fn simple_list3(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<u32>,
                    b: wasmtime::component::__internal::Vec<u32>,
                ) -> (
                    wasmtime::component::__internal::Vec<u32>,
                    wasmtime::component::__internal::Vec<u32>,
                );
                fn simple_list4(
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
                let mut inst = linker.instance("foo:foo/simple-lists")?;
                inst.func_wrap(
                    "simple-list1",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u32>,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list1(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "simple-list2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list2(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
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
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list3(host, arg0, arg1);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
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
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::simple_list4(host, arg0);
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
                fn simple_list1(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<u32>,
                ) -> () {
                    Host::simple_list1(*self, l)
                }
                fn simple_list2(&mut self) -> wasmtime::component::__internal::Vec<u32> {
                    Host::simple_list2(*self)
                }
                fn simple_list3(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<u32>,
                    b: wasmtime::component::__internal::Vec<u32>,
                ) -> (
                    wasmtime::component::__internal::Vec<u32>,
                    wasmtime::component::__internal::Vec<u32>,
                ) {
                    Host::simple_list3(*self, a, b)
                }
                fn simple_list4(
                    &mut self,
                    l: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::Vec<u32>,
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::Vec<u32>,
                > {
                    Host::simple_list4(*self, l)
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
                    pub fn call_simple_list1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u32],
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u32],),
                                (),
                            >::new_unchecked(self.simple_list1)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_simple_list2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u32>> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >::new_unchecked(self.simple_list2)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_simple_list3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u32],
                        arg1: &[u32],
                    ) -> wasmtime::Result<
                        (
                            wasmtime::component::__internal::Vec<u32>,
                            wasmtime::component::__internal::Vec<u32>,
                        ),
                    > {
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
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0, arg1))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_simple_list4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[wasmtime::component::__internal::Vec<u32>],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::Vec<u32>,
                        >,
                    > {
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
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
