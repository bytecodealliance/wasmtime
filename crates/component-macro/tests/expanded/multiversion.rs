/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `foo`.
///
/// This structure is created through [`FooPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct FooPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::my::dep0_1_0::a::GuestPre,
    interface1: exports::my::dep0_2_0::a::GuestPre,
}
impl<T> Clone for FooPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            interface0: self.interface0.clone(),
            interface1: self.interface1.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `foo`.
///
/// This structure is created through either
/// [`Foo::instantiate`] or by first creating
/// a [`FooPre`] followed by using
/// [`FooPre::instantiate`].
pub struct Foo {
    interface0: exports::my::dep0_1_0::a::Guest,
    interface1: exports::my::dep0_2_0::a::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> FooPre<_T> {
        /// Creates a new copy of `FooPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::my::dep0_1_0::a::GuestPre::new(_component)?;
            let interface1 = exports::my::dep0_2_0::a::GuestPre::new(_component)?;
            Ok(FooPre {
                instance_pre,
                interface0,
                interface1,
            })
        }
        /// Instantiates a new instance of [`Foo`] within the
        /// `store` provided.
        ///
        /// This function will use `self` as the pre-instantiated
        /// instance to perform instantiation. Afterwards the preloaded
        /// indices in `self` are used to lookup all exports on the
        /// resulting instance.
        pub fn instantiate(
            &self,
            mut store: impl wasmtime::AsContextMut<Data = _T>,
        ) -> wasmtime::Result<Foo> {
            let mut store = store.as_context_mut();
            let _instance = self.instance_pre.instantiate(&mut store)?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            let interface1 = self.interface1.load(&mut store, &_instance)?;
            Ok(Foo { interface0, interface1 })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
        }
    }
    impl Foo {
        /// Convenience wrapper around [`FooPre::new`] and
        /// [`FooPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Foo> {
            let pre = linker.instantiate_pre(component)?;
            FooPre::new(pre)?.instantiate(store)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: my::dep0_1_0::a::Host + my::dep0_2_0::a::Host,
        {
            my::dep0_1_0::a::add_to_linker(linker, get)?;
            my::dep0_2_0::a::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn my_dep0_1_0_a(&self) -> &exports::my::dep0_1_0::a::Guest {
            &self.interface0
        }
        pub fn my_dep0_2_0_a(&self) -> &exports::my::dep0_2_0::a::Guest {
            &self.interface1
        }
    }
};
pub mod my {
    pub mod dep0_1_0 {
        #[allow(clippy::all)]
        pub mod a {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {
                fn x(&mut self) -> ();
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
                let mut inst = linker.instance("my:dep/a@0.1.0")?;
                inst.func_wrap(
                    "x",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::x(host);
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
                fn x(&mut self) -> () {
                    Host::x(*self)
                }
            }
        }
    }
    pub mod dep0_2_0 {
        #[allow(clippy::all)]
        pub mod a {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {
                fn x(&mut self) -> ();
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
                let mut inst = linker.instance("my:dep/a@0.2.0")?;
                inst.func_wrap(
                    "x",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::x(host);
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
                fn x(&mut self) -> () {
                    Host::x(*self)
                }
            }
        }
    }
}
pub mod exports {
    pub mod my {
        pub mod dep0_1_0 {
            #[allow(clippy::all)]
            pub mod a {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    x: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestPre {
                    x: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "my:dep/a@0.1.0")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `my:dep/a@0.1.0`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `my:dep/a@0.1.0` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let x = _lookup("x")?;
                        Ok(GuestPre { x })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let x = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.x)?
                            .func();
                        Ok(Guest { x })
                    }
                }
                impl Guest {
                    pub fn call_x<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.x)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                }
            }
        }
        pub mod dep0_2_0 {
            #[allow(clippy::all)]
            pub mod a {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub struct Guest {
                    x: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestPre {
                    x: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "my:dep/a@0.2.0")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `my:dep/a@0.2.0`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `my:dep/a@0.2.0` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let x = _lookup("x")?;
                        Ok(GuestPre { x })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let x = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.x)?
                            .func();
                        Ok(Guest { x })
                    }
                }
                impl Guest {
                    pub fn call_x<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.x)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                }
            }
        }
    }
}
