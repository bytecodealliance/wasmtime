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
    interface0: exports::foo::foo::simple::GuestIndices,
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
    interface0: exports::foo::foo::simple::Guest,
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
            let interface0 = exports::foo::foo::simple::GuestIndices::new(_component)?;
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
            let interface0 = exports::foo::foo::simple::GuestIndices::new_instance(
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
            T: Send + foo::foo::simple::Host<Data = T> + 'static,
            U: Send + foo::foo::simple::Host<Data = T>,
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
            use wasmtime::component::__internal::{anyhow, Box};
            pub trait Host {
                type Data;
                fn f1(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn f2(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a: u32,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn f3(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a: u32,
                    b: u32,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn f4(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> u32 + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn f5(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> (u32, u32) + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn f6(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a: u32,
                    b: u32,
                    c: u32,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> (u32, u32, u32) + Send + Sync + 'static,
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
                let mut inst = linker.instance("foo:foo/simple")?;
                inst.func_wrap_concurrent(
                    "f1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::f1(host);
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
                    "f2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| {
                        let host = caller;
                        let r = <G::Host as Host>::f2(host, arg0);
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
                    "f3",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0, arg1): (u32, u32)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::f3(host, arg0, arg1);
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
                    "f4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::f4(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(u32,)> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(u32,)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "f5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::f5(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<((u32, u32),)> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<((u32, u32),)> + Send + Sync,
                                        >,
                                    > + Send + Sync + 'static,
                                >,
                            >
                    },
                )?;
                inst.func_wrap_concurrent(
                    "f6",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0, arg1, arg2): (u32, u32, u32)|
                    {
                        let host = caller;
                        let r = <G::Host as Host>::f6(host, arg0, arg1, arg2);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<((u32, u32, u32),)> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<((u32, u32, u32),)> + Send + Sync,
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
                fn f1(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f1(store)
                }
                fn f2(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a: u32,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f2(store, a)
                }
                fn f3(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a: u32,
                    b: u32,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f3(store, a, b)
                }
                fn f4(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> u32 + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f4(store)
                }
                fn f5(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> (u32, u32) + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f5(store)
                }
                fn f6(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                    a: u32,
                    b: u32,
                    c: u32,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> (u32, u32, u32) + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f6(store, a, b, c)
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
                use wasmtime::component::__internal::{anyhow, Box};
                pub struct Guest {
                    f1: wasmtime::component::Func,
                    f2: wasmtime::component::Func,
                    f3: wasmtime::component::Func,
                    f4: wasmtime::component::Func,
                    f5: wasmtime::component::Func,
                    f6: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    f1: wasmtime::component::ComponentExportIndex,
                    f2: wasmtime::component::ComponentExportIndex,
                    f3: wasmtime::component::ComponentExportIndex,
                    f4: wasmtime::component::ComponentExportIndex,
                    f5: wasmtime::component::ComponentExportIndex,
                    f6: wasmtime::component::ComponentExportIndex,
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
                            .export_index(None, "foo:foo/simple")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/simple`"
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
                            .get_export(&mut store, None, "foo:foo/simple")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/simple`"
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
                                        "instance export `foo:foo/simple` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let f1 = lookup("f1")?;
                        let f2 = lookup("f2")?;
                        let f3 = lookup("f3")?;
                        let f4 = lookup("f4")?;
                        let f5 = lookup("f5")?;
                        let f6 = lookup("f6")?;
                        Ok(GuestIndices {
                            f1,
                            f2,
                            f3,
                            f4,
                            f5,
                            f6,
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
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.f1)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_f2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32,),
                                (),
                            >::new_unchecked(self.f2)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0,))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_f3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                        arg1: u32,
                    ) -> wasmtime::Result<wasmtime::component::Promise<()>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32, u32),
                                (),
                            >::new_unchecked(self.f3)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0, arg1))
                            .await?;
                        Ok(promise)
                    }
                    pub async fn call_f4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<u32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.f4)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_f5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::Promise<(u32, u32)>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((u32, u32),),
                            >::new_unchecked(self.f5)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), ())
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                    pub async fn call_f6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                        arg1: u32,
                        arg2: u32,
                    ) -> wasmtime::Result<wasmtime::component::Promise<(u32, u32, u32)>>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32, u32, u32),
                                ((u32, u32, u32),),
                            >::new_unchecked(self.f6)
                        };
                        let promise = callee
                            .call_concurrent(store.as_context_mut(), (arg0, arg1, arg2))
                            .await?;
                        Ok(promise.map(|(v,)| v))
                    }
                }
            }
        }
    }
}
