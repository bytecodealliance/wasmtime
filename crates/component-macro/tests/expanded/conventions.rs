/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::foo::foo::conventions::GuestPre,
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
    interface0: exports::foo::foo::conventions::Guest,
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
            let interface0 = exports::foo::foo::conventions::GuestPre::new(_component)?;
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
            U: foo::foo::conventions::Host,
        {
            foo::foo::conventions::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_conventions(&self) -> &exports::foo::foo::conventions::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod conventions {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct LudicrousSpeed {
                #[component(name = "how-fast-are-you-going")]
                pub how_fast_are_you_going: u32,
                #[component(name = "i-am-going-extremely-slow")]
                pub i_am_going_extremely_slow: u64,
            }
            impl core::fmt::Debug for LudicrousSpeed {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("LudicrousSpeed")
                        .field("how-fast-are-you-going", &self.how_fast_are_you_going)
                        .field(
                            "i-am-going-extremely-slow",
                            &self.i_am_going_extremely_slow,
                        )
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    16 == < LudicrousSpeed as wasmtime::component::ComponentType
                    >::SIZE32
                );
                assert!(
                    8 == < LudicrousSpeed as wasmtime::component::ComponentType
                    >::ALIGN32
                );
            };
            pub trait Host {
                fn kebab_case(&mut self) -> ();
                fn foo(&mut self, x: LudicrousSpeed) -> ();
                fn function_with_dashes(&mut self) -> ();
                fn function_with_no_weird_characters(&mut self) -> ();
                fn apple(&mut self) -> ();
                fn apple_pear(&mut self) -> ();
                fn apple_pear_grape(&mut self) -> ();
                fn a0(&mut self) -> ();
                /// Comment out identifiers that collide when mapped to snake_case, for now; see
                /// https://github.com/WebAssembly/component-model/issues/118
                /// APPLE: func()
                /// APPLE-pear-GRAPE: func()
                /// apple-PEAR-grape: func()
                fn is_xml(&mut self) -> ();
                fn explicit(&mut self) -> ();
                fn explicit_kebab(&mut self) -> ();
                /// Identifiers with the same name as keywords are quoted.
                fn bool(&mut self) -> ();
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
                let mut inst = linker.instance("foo:foo/conventions")?;
                inst.func_wrap(
                    "kebab-case",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::kebab_case(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "foo",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (LudicrousSpeed,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::foo(host, arg0);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "function-with-dashes",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::function_with_dashes(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "function-with-no-weird-characters",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::function_with_no_weird_characters(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "apple",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::apple(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "apple-pear",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::apple_pear(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "apple-pear-grape",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::apple_pear_grape(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "a0",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::a0(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "is-XML",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::is_xml(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "explicit",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::explicit(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "explicit-kebab",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::explicit_kebab(host);
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "bool",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::bool(host);
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
                fn kebab_case(&mut self) -> () {
                    Host::kebab_case(*self)
                }
                fn foo(&mut self, x: LudicrousSpeed) -> () {
                    Host::foo(*self, x)
                }
                fn function_with_dashes(&mut self) -> () {
                    Host::function_with_dashes(*self)
                }
                fn function_with_no_weird_characters(&mut self) -> () {
                    Host::function_with_no_weird_characters(*self)
                }
                fn apple(&mut self) -> () {
                    Host::apple(*self)
                }
                fn apple_pear(&mut self) -> () {
                    Host::apple_pear(*self)
                }
                fn apple_pear_grape(&mut self) -> () {
                    Host::apple_pear_grape(*self)
                }
                fn a0(&mut self) -> () {
                    Host::a0(*self)
                }
                /// Comment out identifiers that collide when mapped to snake_case, for now; see
                /// https://github.com/WebAssembly/component-model/issues/118
                /// APPLE: func()
                /// APPLE-pear-GRAPE: func()
                /// apple-PEAR-grape: func()
                fn is_xml(&mut self) -> () {
                    Host::is_xml(*self)
                }
                fn explicit(&mut self) -> () {
                    Host::explicit(*self)
                }
                fn explicit_kebab(&mut self) -> () {
                    Host::explicit_kebab(*self)
                }
                /// Identifiers with the same name as keywords are quoted.
                fn bool(&mut self) -> () {
                    Host::bool(*self)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod conventions {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone, Copy)]
                pub struct LudicrousSpeed {
                    #[component(name = "how-fast-are-you-going")]
                    pub how_fast_are_you_going: u32,
                    #[component(name = "i-am-going-extremely-slow")]
                    pub i_am_going_extremely_slow: u64,
                }
                impl core::fmt::Debug for LudicrousSpeed {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("LudicrousSpeed")
                            .field(
                                "how-fast-are-you-going",
                                &self.how_fast_are_you_going,
                            )
                            .field(
                                "i-am-going-extremely-slow",
                                &self.i_am_going_extremely_slow,
                            )
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        16 == < LudicrousSpeed as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        8 == < LudicrousSpeed as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub struct Guest {
                    kebab_case: wasmtime::component::Func,
                    foo: wasmtime::component::Func,
                    function_with_dashes: wasmtime::component::Func,
                    function_with_no_weird_characters: wasmtime::component::Func,
                    apple: wasmtime::component::Func,
                    apple_pear: wasmtime::component::Func,
                    apple_pear_grape: wasmtime::component::Func,
                    a0: wasmtime::component::Func,
                    is_xml: wasmtime::component::Func,
                    explicit: wasmtime::component::Func,
                    explicit_kebab: wasmtime::component::Func,
                    bool: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestPre {
                    kebab_case: wasmtime::component::ComponentExportIndex,
                    foo: wasmtime::component::ComponentExportIndex,
                    function_with_dashes: wasmtime::component::ComponentExportIndex,
                    function_with_no_weird_characters: wasmtime::component::ComponentExportIndex,
                    apple: wasmtime::component::ComponentExportIndex,
                    apple_pear: wasmtime::component::ComponentExportIndex,
                    apple_pear_grape: wasmtime::component::ComponentExportIndex,
                    a0: wasmtime::component::ComponentExportIndex,
                    is_xml: wasmtime::component::ComponentExportIndex,
                    explicit: wasmtime::component::ComponentExportIndex,
                    explicit_kebab: wasmtime::component::ComponentExportIndex,
                    bool: wasmtime::component::ComponentExportIndex,
                }
                impl GuestPre {
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestPre> {
                        let _component = component;
                        let (_, instance) = component
                            .export_index(None, "foo:foo/conventions")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/conventions`"
                                )
                            })?;
                        let _lookup = |name: &str| {
                            _component
                                .export_index(Some(&instance), name)
                                .map(|p| p.1)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/conventions` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let kebab_case = _lookup("kebab-case")?;
                        let foo = _lookup("foo")?;
                        let function_with_dashes = _lookup("function-with-dashes")?;
                        let function_with_no_weird_characters = _lookup(
                            "function-with-no-weird-characters",
                        )?;
                        let apple = _lookup("apple")?;
                        let apple_pear = _lookup("apple-pear")?;
                        let apple_pear_grape = _lookup("apple-pear-grape")?;
                        let a0 = _lookup("a0")?;
                        let is_xml = _lookup("is-XML")?;
                        let explicit = _lookup("explicit")?;
                        let explicit_kebab = _lookup("explicit-kebab")?;
                        let bool = _lookup("bool")?;
                        Ok(GuestPre {
                            kebab_case,
                            foo,
                            function_with_dashes,
                            function_with_no_weird_characters,
                            apple,
                            apple_pear,
                            apple_pear_grape,
                            a0,
                            is_xml,
                            explicit,
                            explicit_kebab,
                            bool,
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
                        let kebab_case = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.kebab_case)?
                            .func();
                        let foo = *_instance
                            .get_typed_func::<
                                (LudicrousSpeed,),
                                (),
                            >(&mut store, &self.foo)?
                            .func();
                        let function_with_dashes = *_instance
                            .get_typed_func::<
                                (),
                                (),
                            >(&mut store, &self.function_with_dashes)?
                            .func();
                        let function_with_no_weird_characters = *_instance
                            .get_typed_func::<
                                (),
                                (),
                            >(&mut store, &self.function_with_no_weird_characters)?
                            .func();
                        let apple = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.apple)?
                            .func();
                        let apple_pear = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.apple_pear)?
                            .func();
                        let apple_pear_grape = *_instance
                            .get_typed_func::<
                                (),
                                (),
                            >(&mut store, &self.apple_pear_grape)?
                            .func();
                        let a0 = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.a0)?
                            .func();
                        let is_xml = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.is_xml)?
                            .func();
                        let explicit = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.explicit)?
                            .func();
                        let explicit_kebab = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.explicit_kebab)?
                            .func();
                        let bool = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.bool)?
                            .func();
                        Ok(Guest {
                            kebab_case,
                            foo,
                            function_with_dashes,
                            function_with_no_weird_characters,
                            apple,
                            apple_pear,
                            apple_pear_grape,
                            a0,
                            is_xml,
                            explicit,
                            explicit_kebab,
                            bool,
                        })
                    }
                }
                impl Guest {
                    pub fn call_kebab_case<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.kebab_case)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_foo<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: LudicrousSpeed,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (LudicrousSpeed,),
                                (),
                            >::new_unchecked(self.foo)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_function_with_dashes<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.function_with_dashes)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_function_with_no_weird_characters<
                        S: wasmtime::AsContextMut,
                    >(&self, mut store: S) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.function_with_no_weird_characters)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_apple<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_apple_pear<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple_pear)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_apple_pear_grape<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple_pear_grape)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_a0<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.a0)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    /// Comment out identifiers that collide when mapped to snake_case, for now; see
                    /// https://github.com/WebAssembly/component-model/issues/118
                    /// APPLE: func()
                    /// APPLE-pear-GRAPE: func()
                    /// apple-PEAR-grape: func()
                    pub fn call_is_xml<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.is_xml)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_explicit<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.explicit)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_explicit_kebab<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.explicit_kebab)
                        };
                        let () = callee.call(store.as_context_mut(), ())?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    /// Identifiers with the same name as keywords are quoted.
                    pub fn call_bool<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.bool)
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
