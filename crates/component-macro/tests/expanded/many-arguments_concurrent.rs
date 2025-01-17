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
    interface0: exports::foo::foo::manyarg::GuestIndices,
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
    interface0: exports::foo::foo::manyarg::Guest,
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
            let interface0 = exports::foo::foo::manyarg::GuestIndices::new(_component)?;
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
            let interface0 = exports::foo::foo::manyarg::GuestIndices::new_instance(
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
            T: Send + foo::foo::manyarg::Host<Data = T> + 'static,
            U: Send + foo::foo::manyarg::Host<Data = T>,
        {
            foo::foo::manyarg::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_manyarg(&self) -> &exports::foo::foo::manyarg::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod manyarg {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct BigStruct {
                #[component(name = "a1")]
                pub a1: wasmtime::component::__internal::String,
                #[component(name = "a2")]
                pub a2: wasmtime::component::__internal::String,
                #[component(name = "a3")]
                pub a3: wasmtime::component::__internal::String,
                #[component(name = "a4")]
                pub a4: wasmtime::component::__internal::String,
                #[component(name = "a5")]
                pub a5: wasmtime::component::__internal::String,
                #[component(name = "a6")]
                pub a6: wasmtime::component::__internal::String,
                #[component(name = "a7")]
                pub a7: wasmtime::component::__internal::String,
                #[component(name = "a8")]
                pub a8: wasmtime::component::__internal::String,
                #[component(name = "a9")]
                pub a9: wasmtime::component::__internal::String,
                #[component(name = "a10")]
                pub a10: wasmtime::component::__internal::String,
                #[component(name = "a11")]
                pub a11: wasmtime::component::__internal::String,
                #[component(name = "a12")]
                pub a12: wasmtime::component::__internal::String,
                #[component(name = "a13")]
                pub a13: wasmtime::component::__internal::String,
                #[component(name = "a14")]
                pub a14: wasmtime::component::__internal::String,
                #[component(name = "a15")]
                pub a15: wasmtime::component::__internal::String,
                #[component(name = "a16")]
                pub a16: wasmtime::component::__internal::String,
                #[component(name = "a17")]
                pub a17: wasmtime::component::__internal::String,
                #[component(name = "a18")]
                pub a18: wasmtime::component::__internal::String,
                #[component(name = "a19")]
                pub a19: wasmtime::component::__internal::String,
                #[component(name = "a20")]
                pub a20: wasmtime::component::__internal::String,
            }
            impl core::fmt::Debug for BigStruct {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("BigStruct")
                        .field("a1", &self.a1)
                        .field("a2", &self.a2)
                        .field("a3", &self.a3)
                        .field("a4", &self.a4)
                        .field("a5", &self.a5)
                        .field("a6", &self.a6)
                        .field("a7", &self.a7)
                        .field("a8", &self.a8)
                        .field("a9", &self.a9)
                        .field("a10", &self.a10)
                        .field("a11", &self.a11)
                        .field("a12", &self.a12)
                        .field("a13", &self.a13)
                        .field("a14", &self.a14)
                        .field("a15", &self.a15)
                        .field("a16", &self.a16)
                        .field("a17", &self.a17)
                        .field("a18", &self.a18)
                        .field("a19", &self.a19)
                        .field("a20", &self.a20)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    160 == < BigStruct as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < BigStruct as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub trait Host {
                type Data;
                fn many_args(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a1: u64,
                    a2: u64,
                    a3: u64,
                    a4: u64,
                    a5: u64,
                    a6: u64,
                    a7: u64,
                    a8: u64,
                    a9: u64,
                    a10: u64,
                    a11: u64,
                    a12: u64,
                    a13: u64,
                    a14: u64,
                    a15: u64,
                    a16: u64,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn big_argument(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: BigStruct,
                ) -> impl ::std::future::Future<
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
                let mut inst = linker.instance("foo:foo/manyarg")?;
                inst.func_wrap_concurrent(
                    "many-args",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                            arg6,
                            arg7,
                            arg8,
                            arg9,
                            arg10,
                            arg11,
                            arg12,
                            arg13,
                            arg14,
                            arg15,
                        ): (
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                        )|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::many_args(
                            host,
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                            arg6,
                            arg7,
                            arg8,
                            arg9,
                            arg10,
                            arg11,
                            arg12,
                            arg13,
                            arg14,
                            arg15,
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
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
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
                    "big-argument",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (BigStruct,)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::big_argument(host, arg0);
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
                            as ::std::pin::Pin<
                                Box<
                                    dyn ::std::future::Future<
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
                fn many_args(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a1: u64,
                    a2: u64,
                    a3: u64,
                    a4: u64,
                    a5: u64,
                    a6: u64,
                    a7: u64,
                    a8: u64,
                    a9: u64,
                    a10: u64,
                    a11: u64,
                    a12: u64,
                    a13: u64,
                    a14: u64,
                    a15: u64,
                    a16: u64,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::many_args(
                        store,
                        a1,
                        a2,
                        a3,
                        a4,
                        a5,
                        a6,
                        a7,
                        a8,
                        a9,
                        a10,
                        a11,
                        a12,
                        a13,
                        a14,
                        a15,
                        a16,
                    )
                }
                fn big_argument(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    x: BigStruct,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::big_argument(store, x)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod manyarg {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::{anyhow, Box};
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct BigStruct {
                    #[component(name = "a1")]
                    pub a1: wasmtime::component::__internal::String,
                    #[component(name = "a2")]
                    pub a2: wasmtime::component::__internal::String,
                    #[component(name = "a3")]
                    pub a3: wasmtime::component::__internal::String,
                    #[component(name = "a4")]
                    pub a4: wasmtime::component::__internal::String,
                    #[component(name = "a5")]
                    pub a5: wasmtime::component::__internal::String,
                    #[component(name = "a6")]
                    pub a6: wasmtime::component::__internal::String,
                    #[component(name = "a7")]
                    pub a7: wasmtime::component::__internal::String,
                    #[component(name = "a8")]
                    pub a8: wasmtime::component::__internal::String,
                    #[component(name = "a9")]
                    pub a9: wasmtime::component::__internal::String,
                    #[component(name = "a10")]
                    pub a10: wasmtime::component::__internal::String,
                    #[component(name = "a11")]
                    pub a11: wasmtime::component::__internal::String,
                    #[component(name = "a12")]
                    pub a12: wasmtime::component::__internal::String,
                    #[component(name = "a13")]
                    pub a13: wasmtime::component::__internal::String,
                    #[component(name = "a14")]
                    pub a14: wasmtime::component::__internal::String,
                    #[component(name = "a15")]
                    pub a15: wasmtime::component::__internal::String,
                    #[component(name = "a16")]
                    pub a16: wasmtime::component::__internal::String,
                    #[component(name = "a17")]
                    pub a17: wasmtime::component::__internal::String,
                    #[component(name = "a18")]
                    pub a18: wasmtime::component::__internal::String,
                    #[component(name = "a19")]
                    pub a19: wasmtime::component::__internal::String,
                    #[component(name = "a20")]
                    pub a20: wasmtime::component::__internal::String,
                }
                impl core::fmt::Debug for BigStruct {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("BigStruct")
                            .field("a1", &self.a1)
                            .field("a2", &self.a2)
                            .field("a3", &self.a3)
                            .field("a4", &self.a4)
                            .field("a5", &self.a5)
                            .field("a6", &self.a6)
                            .field("a7", &self.a7)
                            .field("a8", &self.a8)
                            .field("a9", &self.a9)
                            .field("a10", &self.a10)
                            .field("a11", &self.a11)
                            .field("a12", &self.a12)
                            .field("a13", &self.a13)
                            .field("a14", &self.a14)
                            .field("a15", &self.a15)
                            .field("a16", &self.a16)
                            .field("a17", &self.a17)
                            .field("a18", &self.a18)
                            .field("a19", &self.a19)
                            .field("a20", &self.a20)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        160 == < BigStruct as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < BigStruct as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                pub struct Guest {
                    many_args: wasmtime::component::Func,
                    big_argument: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    many_args: wasmtime::component::ComponentExportIndex,
                    big_argument: wasmtime::component::ComponentExportIndex,
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
                            .export_index(None, "foo:foo/manyarg")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/manyarg`"
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
                            .get_export(&mut store, None, "foo:foo/manyarg")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/manyarg`"
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
                                        "instance export `foo:foo/manyarg` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let many_args = lookup("many-args")?;
                        let big_argument = lookup("big-argument")?;
                        Ok(GuestIndices {
                            many_args,
                            big_argument,
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
                        let many_args = *_instance
                            .get_typed_func::<
                                (
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                ),
                                (),
                            >(&mut store, &self.many_args)?
                            .func();
                        let big_argument = *_instance
                            .get_typed_func::<
                                (&BigStruct,),
                                (),
                            >(&mut store, &self.big_argument)?
                            .func();
                        Ok(Guest { many_args, big_argument })
                    }
                }
                impl Guest {
                    pub async fn call_many_args<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u64,
                        arg1: u64,
                        arg2: u64,
                        arg3: u64,
                        arg4: u64,
                        arg5: u64,
                        arg6: u64,
                        arg7: u64,
                        arg8: u64,
                        arg9: u64,
                        arg10: u64,
                        arg11: u64,
                        arg12: u64,
                        arg13: u64,
                        arg14: u64,
                        arg15: u64,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                ),
                                (),
                            >::new_unchecked(self.many_args)
                        };
                        let promise = callee
                            .call_concurrent(
                                store.as_context_mut(),
                                (
                                    arg0,
                                    arg1,
                                    arg2,
                                    arg3,
                                    arg4,
                                    arg5,
                                    arg6,
                                    arg7,
                                    arg8,
                                    arg9,
                                    arg10,
                                    arg11,
                                    arg12,
                                    arg13,
                                    arg14,
                                    arg15,
                                ),
                            )
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_big_argument<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: BigStruct,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (BigStruct,),
                                (),
                            >::new_unchecked(self.big_argument)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                }
            }
        }
    }
}
