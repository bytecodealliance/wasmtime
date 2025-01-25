/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`TheWorld`] as well.
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: TheWorldIndices,
}
impl<T> Clone for TheWorldPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> TheWorldPre<_T> {
    /// Creates a new copy of `TheWorldPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = TheWorldIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
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
        _T: Send + 'static,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `the-world`.
///
/// This is an implementation detail of [`TheWorldPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`TheWorld`] as well.
#[derive(Clone)]
pub struct TheWorldIndices {
    interface0: exports::foo::foo::conventions::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`TheWorld::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`TheWorldPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`TheWorldPre::instantiate_async`] to
///   create a [`TheWorld`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`TheWorld::new`].
///
/// * You can also access the guts of instantiation through
///   [`TheWorldIndices::new_instance`] followed
///   by [`TheWorldIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct TheWorld {
    interface0: exports::foo::foo::conventions::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorldIndices {
        /// Creates a new copy of `TheWorldIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            let interface0 = exports::foo::foo::conventions::GuestIndices::new(
                _component,
            )?;
            Ok(TheWorldIndices { interface0 })
        }
        /// Creates a new instance of [`TheWorldIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`TheWorld`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`TheWorld`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            let interface0 = exports::foo::foo::conventions::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            Ok(TheWorldIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`TheWorld`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let _instance = instance;
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
            _T: Send + 'static,
        {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`TheWorldIndices::new_instance`] and
        /// [`TheWorldIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let indices = TheWorldIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send + foo::foo::conventions::Host<Data = T> + 'static,
            U: Send + foo::foo::conventions::Host<Data = T>,
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
            use wasmtime::component::__internal::{anyhow, Box};
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
                type Data;
                fn kebab_case(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn foo(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: LudicrousSpeed,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn function_with_dashes(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn function_with_no_weird_characters(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn apple(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn apple_pear(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn apple_pear_grape(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn a0(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                /// Comment out identifiers that collide when mapped to snake_case, for now; see
                ///  https://github.com/WebAssembly/component-model/issues/118
                /// APPLE: func()
                /// APPLE-pear-GRAPE: func()
                /// apple-PEAR-grape: func()
                fn is_xml(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn explicit(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn explicit_kebab(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                /// Identifiers with the same name as keywords are quoted.
                fn bool(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
            }
            pub trait GetHost<
                T,
                D,
            >: Fn(T) -> <Self as GetHost<T, D>>::Host + Send + Sync + Copy + 'static {
                type Host: Host<Data = D> + Send;
            }
            impl<F, T, D, O> GetHost<T, D> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host<Data = D> + Send,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<
                T,
                G: for<'a> GetHost<&'a mut T, T, Host: Host<Data = T> + Send>,
            >(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: G,
            ) -> wasmtime::Result<()>
            where
                T: Send + 'static,
            {
                let mut inst = linker.instance("foo:foo/conventions")?;
                inst.func_wrap_concurrent(
                    "kebab-case",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::kebab_case(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "foo",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (LudicrousSpeed,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::foo(host, arg0);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "function-with-dashes",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::function_with_dashes(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "function-with-no-weird-characters",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::function_with_no_weird_characters(
                            host,
                        );
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "apple",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::apple(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "apple-pear",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::apple_pear(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "apple-pear-grape",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::apple_pear_grape(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "a0",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::a0(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "is-XML",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::is_xml(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "explicit",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::explicit(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "explicit-kebab",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::explicit_kebab(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "bool",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::bool(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok(r)
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<()> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<()> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host<Data = T> + Send,
                T: Send + 'static,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host> Host for &mut _T {
                type Data = _T::Data;
                fn kebab_case(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::kebab_case(store)
                }
                fn foo(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: LudicrousSpeed,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::foo(store, x)
                }
                fn function_with_dashes(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::function_with_dashes(store)
                }
                fn function_with_no_weird_characters(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::function_with_no_weird_characters(store)
                }
                fn apple(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::apple(store)
                }
                fn apple_pear(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::apple_pear(store)
                }
                fn apple_pear_grape(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::apple_pear_grape(store)
                }
                fn a0(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::a0(store)
                }
                /// Comment out identifiers that collide when mapped to snake_case, for now; see
                ///  https://github.com/WebAssembly/component-model/issues/118
                /// APPLE: func()
                /// APPLE-pear-GRAPE: func()
                /// apple-PEAR-grape: func()
                fn is_xml(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::is_xml(store)
                }
                fn explicit(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::explicit(store)
                }
                fn explicit_kebab(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::explicit_kebab(store)
                }
                /// Identifiers with the same name as keywords are quoted.
                fn bool(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::bool(store)
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
                use wasmtime::component::__internal::{anyhow, Box};
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
                pub struct GuestIndices {
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
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new(
                        component: &wasmtime::component::Component,
                    ) -> wasmtime::Result<GuestIndices> {
                        let (_, instance) = component
                            .export_index(None, "foo:foo/conventions")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/conventions`"
                                )
                            })?;
                        Self::_new(|name| {
                            component.export_index(Some(&instance), name).map(|p| p.1)
                        })
                    }
                    /// This constructor is similar to [`GuestIndices::new`] except that it
                    /// performs string lookups after instantiation time.
                    pub fn new_instance(
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance_export = instance
                            .get_export(&mut store, None, "foo:foo/conventions")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/conventions`"
                                )
                            })?;
                        Self::_new(|name| {
                            instance.get_export(&mut store, Some(&instance_export), name)
                        })
                    }
                    fn _new(
                        mut lookup: impl FnMut(
                            &str,
                        ) -> Option<wasmtime::component::ComponentExportIndex>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let mut lookup = move |name| {
                            lookup(name)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/conventions` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let kebab_case = lookup("kebab-case")?;
                        let foo = lookup("foo")?;
                        let function_with_dashes = lookup("function-with-dashes")?;
                        let function_with_no_weird_characters = lookup(
                            "function-with-no-weird-characters",
                        )?;
                        let apple = lookup("apple")?;
                        let apple_pear = lookup("apple-pear")?;
                        let apple_pear_grape = lookup("apple-pear-grape")?;
                        let a0 = lookup("a0")?;
                        let is_xml = lookup("is-XML")?;
                        let explicit = lookup("explicit")?;
                        let explicit_kebab = lookup("explicit-kebab")?;
                        let bool = lookup("bool")?;
                        Ok(GuestIndices {
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
                    pub async fn call_kebab_case<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.kebab_case)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_foo<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: LudicrousSpeed,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (LudicrousSpeed,),
                                (),
                            >::new_unchecked(self.foo)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_function_with_dashes<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.function_with_dashes)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_function_with_no_weird_characters<
                        S: wasmtime::AsContextMut,
                    >(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.function_with_no_weird_characters)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_apple<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_apple_pear<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple_pear)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_apple_pear_grape<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.apple_pear_grape)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_a0<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.a0)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    /// Comment out identifiers that collide when mapped to snake_case, for now; see
                    ///  https://github.com/WebAssembly/component-model/issues/118
                    /// APPLE: func()
                    /// APPLE-pear-GRAPE: func()
                    /// apple-PEAR-grape: func()
                    pub async fn call_is_xml<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.is_xml)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_explicit<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.explicit)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_explicit_kebab<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.explicit_kebab)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    /// Identifiers with the same name as keywords are quoted.
                    pub async fn call_bool<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.bool)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                }
            }
        }
    }
}
