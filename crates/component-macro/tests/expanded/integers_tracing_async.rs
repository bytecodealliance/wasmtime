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
        _T: Send,
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
    interface0: exports::foo::foo::integers::GuestIndices,
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
    interface0: exports::foo::foo::integers::Guest,
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
            let interface0 = exports::foo::foo::integers::GuestIndices::new(_component)?;
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
            let interface0 = exports::foo::foo::integers::GuestIndices::new_instance(
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
            _T: Send,
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
            T: Send,
            U: foo::foo::integers::Host + Send,
        {
            foo::foo::integers::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_integers(&self) -> &exports::foo::foo::integers::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod integers {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
            pub trait Host: Send {
                async fn a1(&mut self, x: u8) -> ();
                async fn a2(&mut self, x: i8) -> ();
                async fn a3(&mut self, x: u16) -> ();
                async fn a4(&mut self, x: i16) -> ();
                async fn a5(&mut self, x: u32) -> ();
                async fn a6(&mut self, x: i32) -> ();
                async fn a7(&mut self, x: u64) -> ();
                async fn a8(&mut self, x: i64) -> ();
                async fn a9(
                    &mut self,
                    p1: u8,
                    p2: i8,
                    p3: u16,
                    p4: i16,
                    p5: u32,
                    p6: i32,
                    p7: u64,
                    p8: i64,
                ) -> ();
                async fn r1(&mut self) -> u8;
                async fn r2(&mut self) -> i8;
                async fn r3(&mut self) -> u16;
                async fn r4(&mut self) -> i16;
                async fn r5(&mut self) -> u32;
                async fn r6(&mut self) -> i32;
                async fn r7(&mut self) -> u64;
                async fn r8(&mut self) -> i64;
                async fn pair_ret(&mut self) -> (i64, u8);
            }
            pub trait GetHost<
                T,
                D,
            >: Fn(T) -> <Self as GetHost<T, D>>::Host + Send + Sync + Copy + 'static {
                type Host: Host + Send;
            }
            impl<F, T, D, O> GetHost<T, D> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host + Send,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<
                T,
                G: for<'a> GetHost<&'a mut T, T, Host: Host + Send>,
            >(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: G,
            ) -> wasmtime::Result<()>
            where
                T: Send,
            {
                let mut inst = linker.instance("foo:foo/integers")?;
                inst.func_wrap_async(
                    "a1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u8,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a1",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a1(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i8,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a2",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a2(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u16,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a3",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a3(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i16,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a4",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a4(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a5",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a5(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a6",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i32,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a6",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a6(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a7",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u64,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a7",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a7(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a8",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (i64,)| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a8",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a8(host, arg0).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "a9",
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
                        ): (u8, i8, u16, i16, u32, i32, u64, i64)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "a9",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, p1 = tracing::field::debug(& arg0),
                                    p2 = tracing::field::debug(& arg1), p3 =
                                    tracing::field::debug(& arg2), p4 = tracing::field::debug(&
                                    arg3), p5 = tracing::field::debug(& arg4), p6 =
                                    tracing::field::debug(& arg5), p7 = tracing::field::debug(&
                                    arg6), p8 = tracing::field::debug(& arg7), "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::a9(
                                        host,
                                        arg0,
                                        arg1,
                                        arg2,
                                        arg3,
                                        arg4,
                                        arg5,
                                        arg6,
                                        arg7,
                                    )
                                    .await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok(r)
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r1",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r1",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r1(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r2",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r2",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r2(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r3",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r3",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r3(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r4",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r4",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r4(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r5",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r5",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r5(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r6",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r6",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r6(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r7",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r7",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r7(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "r8",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "r8",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::r8(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
                        )
                    },
                )?;
                inst.func_wrap_async(
                    "pair-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "integers", function = "pair-ret",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::pair_ret(host).await;
                                tracing::event!(
                                    tracing::Level::TRACE, result = tracing::field::debug(& r),
                                    "return"
                                );
                                Ok((r,))
                            }
                                .instrument(span),
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                async fn a1(&mut self, x: u8) -> () {
                    Host::a1(*self, x).await
                }
                async fn a2(&mut self, x: i8) -> () {
                    Host::a2(*self, x).await
                }
                async fn a3(&mut self, x: u16) -> () {
                    Host::a3(*self, x).await
                }
                async fn a4(&mut self, x: i16) -> () {
                    Host::a4(*self, x).await
                }
                async fn a5(&mut self, x: u32) -> () {
                    Host::a5(*self, x).await
                }
                async fn a6(&mut self, x: i32) -> () {
                    Host::a6(*self, x).await
                }
                async fn a7(&mut self, x: u64) -> () {
                    Host::a7(*self, x).await
                }
                async fn a8(&mut self, x: i64) -> () {
                    Host::a8(*self, x).await
                }
                async fn a9(
                    &mut self,
                    p1: u8,
                    p2: i8,
                    p3: u16,
                    p4: i16,
                    p5: u32,
                    p6: i32,
                    p7: u64,
                    p8: i64,
                ) -> () {
                    Host::a9(*self, p1, p2, p3, p4, p5, p6, p7, p8).await
                }
                async fn r1(&mut self) -> u8 {
                    Host::r1(*self).await
                }
                async fn r2(&mut self) -> i8 {
                    Host::r2(*self).await
                }
                async fn r3(&mut self) -> u16 {
                    Host::r3(*self).await
                }
                async fn r4(&mut self) -> i16 {
                    Host::r4(*self).await
                }
                async fn r5(&mut self) -> u32 {
                    Host::r5(*self).await
                }
                async fn r6(&mut self) -> i32 {
                    Host::r6(*self).await
                }
                async fn r7(&mut self) -> u64 {
                    Host::r7(*self).await
                }
                async fn r8(&mut self) -> i64 {
                    Host::r8(*self).await
                }
                async fn pair_ret(&mut self) -> (i64, u8) {
                    Host::pair_ret(*self).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod integers {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::{anyhow, Box};
                pub struct Guest {
                    a1: wasmtime::component::Func,
                    a2: wasmtime::component::Func,
                    a3: wasmtime::component::Func,
                    a4: wasmtime::component::Func,
                    a5: wasmtime::component::Func,
                    a6: wasmtime::component::Func,
                    a7: wasmtime::component::Func,
                    a8: wasmtime::component::Func,
                    a9: wasmtime::component::Func,
                    r1: wasmtime::component::Func,
                    r2: wasmtime::component::Func,
                    r3: wasmtime::component::Func,
                    r4: wasmtime::component::Func,
                    r5: wasmtime::component::Func,
                    r6: wasmtime::component::Func,
                    r7: wasmtime::component::Func,
                    r8: wasmtime::component::Func,
                    pair_ret: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    a1: wasmtime::component::ComponentExportIndex,
                    a2: wasmtime::component::ComponentExportIndex,
                    a3: wasmtime::component::ComponentExportIndex,
                    a4: wasmtime::component::ComponentExportIndex,
                    a5: wasmtime::component::ComponentExportIndex,
                    a6: wasmtime::component::ComponentExportIndex,
                    a7: wasmtime::component::ComponentExportIndex,
                    a8: wasmtime::component::ComponentExportIndex,
                    a9: wasmtime::component::ComponentExportIndex,
                    r1: wasmtime::component::ComponentExportIndex,
                    r2: wasmtime::component::ComponentExportIndex,
                    r3: wasmtime::component::ComponentExportIndex,
                    r4: wasmtime::component::ComponentExportIndex,
                    r5: wasmtime::component::ComponentExportIndex,
                    r6: wasmtime::component::ComponentExportIndex,
                    r7: wasmtime::component::ComponentExportIndex,
                    r8: wasmtime::component::ComponentExportIndex,
                    pair_ret: wasmtime::component::ComponentExportIndex,
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
                            .export_index(None, "foo:foo/integers")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/integers`"
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
                            .get_export(&mut store, None, "foo:foo/integers")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/integers`"
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
                                        "instance export `foo:foo/integers` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let a1 = lookup("a1")?;
                        let a2 = lookup("a2")?;
                        let a3 = lookup("a3")?;
                        let a4 = lookup("a4")?;
                        let a5 = lookup("a5")?;
                        let a6 = lookup("a6")?;
                        let a7 = lookup("a7")?;
                        let a8 = lookup("a8")?;
                        let a9 = lookup("a9")?;
                        let r1 = lookup("r1")?;
                        let r2 = lookup("r2")?;
                        let r3 = lookup("r3")?;
                        let r4 = lookup("r4")?;
                        let r5 = lookup("r5")?;
                        let r6 = lookup("r6")?;
                        let r7 = lookup("r7")?;
                        let r8 = lookup("r8")?;
                        let pair_ret = lookup("pair-ret")?;
                        Ok(GuestIndices {
                            a1,
                            a2,
                            a3,
                            a4,
                            a5,
                            a6,
                            a7,
                            a8,
                            a9,
                            r1,
                            r2,
                            r3,
                            r4,
                            r5,
                            r6,
                            r7,
                            r8,
                            pair_ret,
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
                        let a1 = *_instance
                            .get_typed_func::<(u8,), ()>(&mut store, &self.a1)?
                            .func();
                        let a2 = *_instance
                            .get_typed_func::<(i8,), ()>(&mut store, &self.a2)?
                            .func();
                        let a3 = *_instance
                            .get_typed_func::<(u16,), ()>(&mut store, &self.a3)?
                            .func();
                        let a4 = *_instance
                            .get_typed_func::<(i16,), ()>(&mut store, &self.a4)?
                            .func();
                        let a5 = *_instance
                            .get_typed_func::<(u32,), ()>(&mut store, &self.a5)?
                            .func();
                        let a6 = *_instance
                            .get_typed_func::<(i32,), ()>(&mut store, &self.a6)?
                            .func();
                        let a7 = *_instance
                            .get_typed_func::<(u64,), ()>(&mut store, &self.a7)?
                            .func();
                        let a8 = *_instance
                            .get_typed_func::<(i64,), ()>(&mut store, &self.a8)?
                            .func();
                        let a9 = *_instance
                            .get_typed_func::<
                                (u8, i8, u16, i16, u32, i32, u64, i64),
                                (),
                            >(&mut store, &self.a9)?
                            .func();
                        let r1 = *_instance
                            .get_typed_func::<(), (u8,)>(&mut store, &self.r1)?
                            .func();
                        let r2 = *_instance
                            .get_typed_func::<(), (i8,)>(&mut store, &self.r2)?
                            .func();
                        let r3 = *_instance
                            .get_typed_func::<(), (u16,)>(&mut store, &self.r3)?
                            .func();
                        let r4 = *_instance
                            .get_typed_func::<(), (i16,)>(&mut store, &self.r4)?
                            .func();
                        let r5 = *_instance
                            .get_typed_func::<(), (u32,)>(&mut store, &self.r5)?
                            .func();
                        let r6 = *_instance
                            .get_typed_func::<(), (i32,)>(&mut store, &self.r6)?
                            .func();
                        let r7 = *_instance
                            .get_typed_func::<(), (u64,)>(&mut store, &self.r7)?
                            .func();
                        let r8 = *_instance
                            .get_typed_func::<(), (i64,)>(&mut store, &self.r8)?
                            .func();
                        let pair_ret = *_instance
                            .get_typed_func::<
                                (),
                                ((i64, u8),),
                            >(&mut store, &self.pair_ret)?
                            .func();
                        Ok(Guest {
                            a1,
                            a2,
                            a3,
                            a4,
                            a5,
                            a6,
                            a7,
                            a8,
                            a9,
                            r1,
                            r2,
                            r3,
                            r4,
                            r5,
                            r6,
                            r7,
                            r8,
                            pair_ret,
                        })
                    }
                }
                impl Guest {
                    pub async fn call_a1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u8,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a1",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u8,),
                                (),
                            >::new_unchecked(self.a1)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i8,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a2",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i8,),
                                (),
                            >::new_unchecked(self.a2)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u16,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a3",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u16,),
                                (),
                            >::new_unchecked(self.a3)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i16,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a4",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i16,),
                                (),
                            >::new_unchecked(self.a4)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u32,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a5",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u32,),
                                (),
                            >::new_unchecked(self.a5)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i32,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a6",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i32,),
                                (),
                            >::new_unchecked(self.a6)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a7<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u64,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a7",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u64,),
                                (),
                            >::new_unchecked(self.a7)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: i64,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a8",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (i64,),
                                (),
                            >::new_unchecked(self.a8)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_a9<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u8,
                        arg1: i8,
                        arg2: u16,
                        arg3: i16,
                        arg4: u32,
                        arg5: i32,
                        arg6: u64,
                        arg7: i64,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "a9",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (u8, i8, u16, i16, u32, i32, u64, i64),
                                (),
                            >::new_unchecked(self.a9)
                        };
                        let () = callee
                            .call_async(
                                store.as_context_mut(),
                                (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7),
                            )
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(())
                    }
                    pub async fn call_r1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u8>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r1",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u8,),
                            >::new_unchecked(self.r1)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i8>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r2",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i8,),
                            >::new_unchecked(self.r2)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r3<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u16>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r3",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u16,),
                            >::new_unchecked(self.r3)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i16>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r4",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i16,),
                            >::new_unchecked(self.r4)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r5<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u32>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r5",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u32,),
                            >::new_unchecked(self.r5)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r6<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i32>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r6",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i32,),
                            >::new_unchecked(self.r6)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r7<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<u64>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r7",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (u64,),
                            >::new_unchecked(self.r7)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_r8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<i64>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "r8",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (i64,),
                            >::new_unchecked(self.r8)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                    pub async fn call_pair_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<(i64, u8)>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/integers", function = "pair-ret",
                        );
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                ((i64, u8),),
                            >::new_unchecked(self.pair_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .instrument(span.clone())
                            .await?;
                        callee
                            .post_return_async(store.as_context_mut())
                            .instrument(span)
                            .await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
