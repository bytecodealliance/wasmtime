/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::foo::foo::multi_return::GuestPre,
}
impl<T> Clone for TheWorldPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            interface0: self.interface0.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure is created through either
/// [`TheWorld::instantiate`] or by first creating
/// a [`TheWorldPre`] followed by using
/// [`TheWorldPre::instantiate`].
pub struct TheWorld {
    interface0: exports::foo::foo::multi_return::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> TheWorldPre<_T> {
        /// Creates a new copy of `TheWorldPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::foo::foo::multi_return::GuestPre::new(_component)?;
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
        pub fn instantiate(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<TheWorld> {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate(&mut store)?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheWorld { interface0 })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl TheWorld {
        /// Convenience wrapper around [`TheWorldPre::new`] and
        /// [`TheWorldPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheWorld> {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate(store)
        }
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
                let mut inst = linker.instance("foo:foo/multi-return")?;
                inst.func_wrap(
                    "mra",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::mra(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "mrb",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::mrb(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "mrc",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::mrc(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "mrd",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::mrd(host);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "mre",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::mre(host);
                        Ok(r)
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
                #[derive(Clone)]
                pub struct GuestPre {
                    mra: wasmtime::component::ComponentExportIndex,
                    mrb: wasmtime::component::ComponentExportIndex,
                    mrc: wasmtime::component::ComponentExportIndex,
                    mrd: wasmtime::component::ComponentExportIndex,
                    mre: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/multi-return")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/multi-return`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/multi-return` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let mra = _lookup("mra")?;
                        let mrb = _lookup("mrb")?;
                        let mrc = _lookup("mrc")?;
                        let mrd = _lookup("mrd")?;
                        let mre = _lookup("mre")?;
                        Ok(GuestPre {
                            mra,
                            mrb,
                            mrc,
                            mrd,
                            mre,
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
                        let mra = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.mra)?
                            .func();
                        let mrb = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.mrb)?
                            .func();
                        let mrc = *_instance
                            .get_typed_func::<(), (u32,)>(&mut store, &self.mrc)?
                            .func();
                        let mrd = *_instance
                            .get_typed_func::<(), (u32,)>(&mut store, &self.mrd)?
                            .func();
                        let mre = *_instance
                            .get_typed_func::<(), (u32, f32)>(&mut store, &self.mre)?
                            .func();
                        Ok(Guest { mra, mrb, mrc, mrd, mre })
                    }
                }
                impl Guest {
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
