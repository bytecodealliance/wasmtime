pub enum WorldResource {}
#[wasmtime::component::__internal::async_trait]
pub trait HostWorldResource {
    async fn new(&mut self) -> wasmtime::component::Resource<WorldResource>;
    async fn foo(&mut self, self_: wasmtime::component::Resource<WorldResource>) -> ();
    async fn static_foo(&mut self) -> ();
    async fn drop(
        &mut self,
        rep: wasmtime::component::Resource<WorldResource>,
    ) -> wasmtime::Result<()>;
}
#[wasmtime::component::__internal::async_trait]
impl<_T: HostWorldResource + ?Sized + Send> HostWorldResource for &mut _T {
    async fn new(&mut self) -> wasmtime::component::Resource<WorldResource> {
        HostWorldResource::new(*self).await
    }
    async fn foo(&mut self, self_: wasmtime::component::Resource<WorldResource>) -> () {
        HostWorldResource::foo(*self, self_).await
    }
    async fn static_foo(&mut self) -> () {
        HostWorldResource::static_foo(*self).await
    }
    async fn drop(
        &mut self,
        rep: wasmtime::component::Resource<WorldResource>,
    ) -> wasmtime::Result<()> {
        HostWorldResource::drop(*self, rep).await
    }
}
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
    interface1: exports::foo::foo::uses_resource_transitively::GuestIndices,
    some_world_func2: wasmtime::component::ComponentExportIndex,
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
    interface1: exports::foo::foo::uses_resource_transitively::Guest,
    some_world_func2: wasmtime::component::Func,
}
#[wasmtime::component::__internal::async_trait]
pub trait TheWorldImports: Send + HostWorldResource {
    async fn some_world_func(&mut self) -> wasmtime::component::Resource<WorldResource>;
}
pub trait TheWorldImportsGetHost<
    T,
>: Fn(T) -> <Self as TheWorldImportsGetHost<T>>::Host + Send + Sync + Copy + 'static {
    type Host: TheWorldImports;
}
impl<F, T, O> TheWorldImportsGetHost<T> for F
where
    F: Fn(T) -> O + Send + Sync + Copy + 'static,
    O: TheWorldImports,
{
    type Host = O;
}
#[wasmtime::component::__internal::async_trait]
impl<_T: TheWorldImports + ?Sized + Send> TheWorldImports for &mut _T {
    async fn some_world_func(&mut self) -> wasmtime::component::Resource<WorldResource> {
        TheWorldImports::some_world_func(*self).await
    }
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
            let interface1 = exports::foo::foo::uses_resource_transitively::GuestIndices::new(
                _component,
            )?;
            let some_world_func2 = _component
                .export_index(None, "some-world-func2")
                .ok_or_else(|| {
                    anyhow::anyhow!("no function export `some-world-func2` found")
                })?
                .1;
            Ok(TheWorldIndices {
                interface1,
                some_world_func2,
            })
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
            let interface1 = exports::foo::foo::uses_resource_transitively::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            let some_world_func2 = _instance
                .get_export(&mut store, None, "some-world-func2")
                .ok_or_else(|| {
                    anyhow::anyhow!("no function export `some-world-func2` found")
                })?;
            Ok(TheWorldIndices {
                interface1,
                some_world_func2,
            })
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
            let interface1 = self.interface1.load(&mut store, &_instance)?;
            let some_world_func2 = *_instance
                .get_typed_func::<
                    (),
                    (wasmtime::component::Resource<WorldResource>,),
                >(&mut store, &self.some_world_func2)?
                .func();
            Ok(TheWorld {
                interface1,
                some_world_func2,
            })
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
        pub fn add_to_linker_imports_get_host<T>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: impl for<'a> TheWorldImportsGetHost<&'a mut T>,
        ) -> wasmtime::Result<()>
        where
            T: Send,
        {
            let mut linker = linker.root();
            linker
                .resource_async(
                    "world-resource",
                    wasmtime::component::ResourceType::host::<WorldResource>(),
                    move |mut store, rep| {
                        std::boxed::Box::new(async move {
                            HostWorldResource::drop(
                                    &mut host_getter(store.data_mut()),
                                    wasmtime::component::Resource::new_own(rep),
                                )
                                .await
                        })
                    },
                )?;
            linker
                .func_wrap_async(
                    "[constructor]world-resource",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "the-world", function = "[constructor]world-resource",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = HostWorldResource::new(host).await;
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
            linker
                .func_wrap_async(
                    "[method]world-resource.foo",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::Resource<WorldResource>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "the-world", function = "[method]world-resource.foo",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, self_ = tracing::field::debug(&
                                    arg0), "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = HostWorldResource::foo(host, arg0).await;
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
            linker
                .func_wrap_async(
                    "[static]world-resource.static-foo",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "the-world", function = "[static]world-resource.static-foo",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = HostWorldResource::static_foo(host).await;
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
            linker
                .func_wrap_async(
                    "some-world-func",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "the-world", function = "some-world-func",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = TheWorldImports::some_world_func(host).await;
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
            T: Send,
            U: foo::foo::resources::Host + foo::foo::long_use_chain1::Host
                + foo::foo::long_use_chain2::Host + foo::foo::long_use_chain3::Host
                + foo::foo::long_use_chain4::Host
                + foo::foo::transitive_interface_with_resource::Host + TheWorldImports
                + Send,
        {
            Self::add_to_linker_imports_get_host(linker, get)?;
            foo::foo::resources::add_to_linker(linker, get)?;
            foo::foo::long_use_chain1::add_to_linker(linker, get)?;
            foo::foo::long_use_chain2::add_to_linker(linker, get)?;
            foo::foo::long_use_chain3::add_to_linker(linker, get)?;
            foo::foo::long_use_chain4::add_to_linker(linker, get)?;
            foo::foo::transitive_interface_with_resource::add_to_linker(linker, get)?;
            Ok(())
        }
        pub async fn call_some_world_func2<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
        ) -> wasmtime::Result<wasmtime::component::Resource<WorldResource>>
        where
            <S as wasmtime::AsContext>::Data: Send,
        {
            let span = tracing::span!(
                tracing::Level::TRACE, "wit-bindgen export", module = "default", function
                = "some-world-func2",
            );
            let _enter = span.enter();
            let callee = unsafe {
                wasmtime::component::TypedFunc::<
                    (),
                    (wasmtime::component::Resource<WorldResource>,),
                >::new_unchecked(self.some_world_func2)
            };
            let (ret0,) = callee.call_async(store.as_context_mut(), ()).await?;
            callee.post_return_async(store.as_context_mut()).await?;
            Ok(ret0)
        }
        pub fn foo_foo_uses_resource_transitively(
            &self,
        ) -> &exports::foo::foo::uses_resource_transitively::Guest {
            &self.interface1
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod resources {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub enum Bar {}
            #[wasmtime::component::__internal::async_trait]
            pub trait HostBar {
                async fn new(&mut self) -> wasmtime::component::Resource<Bar>;
                async fn static_a(&mut self) -> u32;
                async fn method_a(
                    &mut self,
                    self_: wasmtime::component::Resource<Bar>,
                ) -> u32;
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Bar>,
                ) -> wasmtime::Result<()>;
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: HostBar + ?Sized + Send> HostBar for &mut _T {
                async fn new(&mut self) -> wasmtime::component::Resource<Bar> {
                    HostBar::new(*self).await
                }
                async fn static_a(&mut self) -> u32 {
                    HostBar::static_a(*self).await
                }
                async fn method_a(
                    &mut self,
                    self_: wasmtime::component::Resource<Bar>,
                ) -> u32 {
                    HostBar::method_a(*self, self_).await
                }
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Bar>,
                ) -> wasmtime::Result<()> {
                    HostBar::drop(*self, rep).await
                }
            }
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            pub struct NestedOwn {
                #[component(name = "nested-bar")]
                pub nested_bar: wasmtime::component::Resource<Bar>,
            }
            impl core::fmt::Debug for NestedOwn {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("NestedOwn")
                        .field("nested-bar", &self.nested_bar)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    4 == < NestedOwn as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < NestedOwn as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            pub struct NestedBorrow {
                #[component(name = "nested-bar")]
                pub nested_bar: wasmtime::component::Resource<Bar>,
            }
            impl core::fmt::Debug for NestedBorrow {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("NestedBorrow")
                        .field("nested-bar", &self.nested_bar)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    4 == < NestedBorrow as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < NestedBorrow as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub type SomeHandle = wasmtime::component::Resource<Bar>;
            const _: () = {
                assert!(
                    4 == < SomeHandle as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < SomeHandle as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send + HostBar {
                async fn bar_own_arg(
                    &mut self,
                    x: wasmtime::component::Resource<Bar>,
                ) -> ();
                async fn bar_borrow_arg(
                    &mut self,
                    x: wasmtime::component::Resource<Bar>,
                ) -> ();
                async fn bar_result(&mut self) -> wasmtime::component::Resource<Bar>;
                async fn tuple_own_arg(
                    &mut self,
                    x: (wasmtime::component::Resource<Bar>, u32),
                ) -> ();
                async fn tuple_borrow_arg(
                    &mut self,
                    x: (wasmtime::component::Resource<Bar>, u32),
                ) -> ();
                async fn tuple_result(
                    &mut self,
                ) -> (wasmtime::component::Resource<Bar>, u32);
                async fn option_own_arg(
                    &mut self,
                    x: Option<wasmtime::component::Resource<Bar>>,
                ) -> ();
                async fn option_borrow_arg(
                    &mut self,
                    x: Option<wasmtime::component::Resource<Bar>>,
                ) -> ();
                async fn option_result(
                    &mut self,
                ) -> Option<wasmtime::component::Resource<Bar>>;
                async fn result_own_arg(
                    &mut self,
                    x: Result<wasmtime::component::Resource<Bar>, ()>,
                ) -> ();
                async fn result_borrow_arg(
                    &mut self,
                    x: Result<wasmtime::component::Resource<Bar>, ()>,
                ) -> ();
                async fn result_result(
                    &mut self,
                ) -> Result<wasmtime::component::Resource<Bar>, ()>;
                async fn list_own_arg(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::Resource<Bar>,
                    >,
                ) -> ();
                async fn list_borrow_arg(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::Resource<Bar>,
                    >,
                ) -> ();
                async fn list_result(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::Resource<Bar>,
                >;
                async fn record_own_arg(&mut self, x: NestedOwn) -> ();
                async fn record_borrow_arg(&mut self, x: NestedBorrow) -> ();
                async fn record_result(&mut self) -> NestedOwn;
                async fn func_with_handle_typedef(&mut self, x: SomeHandle) -> ();
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
                let mut inst = linker.instance("foo:foo/resources")?;
                inst.resource_async(
                    "bar",
                    wasmtime::component::ResourceType::host::<Bar>(),
                    move |mut store, rep| {
                        std::boxed::Box::new(async move {
                            HostBar::drop(
                                    &mut host_getter(store.data_mut()),
                                    wasmtime::component::Resource::new_own(rep),
                                )
                                .await
                        })
                    },
                )?;
                inst.func_wrap_async(
                    "[constructor]bar",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "[constructor]bar",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = HostBar::new(host).await;
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
                    "[static]bar.static-a",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "[static]bar.static-a",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = HostBar::static_a(host).await;
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
                    "[method]bar.method-a",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::Resource<Bar>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "[method]bar.method-a",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, self_ = tracing::field::debug(&
                                    arg0), "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = HostBar::method_a(host, arg0).await;
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
                    "bar-own-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::Resource<Bar>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "bar-own-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::bar_own_arg(host, arg0).await;
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
                    "bar-borrow-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::Resource<Bar>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "bar-borrow-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::bar_borrow_arg(host, arg0).await;
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
                    "bar-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "bar-result",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::bar_result(host).await;
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
                    "tuple-own-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): ((wasmtime::component::Resource<Bar>, u32),)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "tuple-own-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::tuple_own_arg(host, arg0).await;
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
                    "tuple-borrow-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): ((wasmtime::component::Resource<Bar>, u32),)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "tuple-borrow-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::tuple_borrow_arg(host, arg0).await;
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
                    "tuple-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "tuple-result",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::tuple_result(host).await;
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
                    "option-own-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Option<wasmtime::component::Resource<Bar>>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "option-own-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::option_own_arg(host, arg0).await;
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
                    "option-borrow-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Option<wasmtime::component::Resource<Bar>>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "option-borrow-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::option_borrow_arg(host, arg0).await;
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
                    "option-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "option-result",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::option_result(host).await;
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
                    "result-own-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Result<wasmtime::component::Resource<Bar>, ()>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "result-own-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::result_own_arg(host, arg0).await;
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
                    "result-borrow-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Result<wasmtime::component::Resource<Bar>, ()>,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "result-borrow-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::result_borrow_arg(host, arg0).await;
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
                    "result-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "result-result",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::result_result(host).await;
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
                    "list-own-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::Resource<Bar>,
                            >,
                        )|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "list-own-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_own_arg(host, arg0).await;
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
                    "list-borrow-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::Resource<Bar>,
                            >,
                        )|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "list-borrow-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_borrow_arg(host, arg0).await;
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
                    "list-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "list-result",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::list_result(host).await;
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
                    "record-own-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (NestedOwn,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "record-own-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::record_own_arg(host, arg0).await;
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
                    "record-borrow-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (NestedBorrow,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "record-borrow-arg",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::record_borrow_arg(host, arg0).await;
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
                    "record-result",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "record-result",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::record_result(host).await;
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
                    "func-with-handle-typedef",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (SomeHandle,)|
                    {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "resources", function = "func-with-handle-typedef",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(
                                    tracing::Level::TRACE, x = tracing::field::debug(& arg0),
                                    "call"
                                );
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::func_with_handle_typedef(host, arg0).await;
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
                async fn bar_own_arg(
                    &mut self,
                    x: wasmtime::component::Resource<Bar>,
                ) -> () {
                    Host::bar_own_arg(*self, x).await
                }
                async fn bar_borrow_arg(
                    &mut self,
                    x: wasmtime::component::Resource<Bar>,
                ) -> () {
                    Host::bar_borrow_arg(*self, x).await
                }
                async fn bar_result(&mut self) -> wasmtime::component::Resource<Bar> {
                    Host::bar_result(*self).await
                }
                async fn tuple_own_arg(
                    &mut self,
                    x: (wasmtime::component::Resource<Bar>, u32),
                ) -> () {
                    Host::tuple_own_arg(*self, x).await
                }
                async fn tuple_borrow_arg(
                    &mut self,
                    x: (wasmtime::component::Resource<Bar>, u32),
                ) -> () {
                    Host::tuple_borrow_arg(*self, x).await
                }
                async fn tuple_result(
                    &mut self,
                ) -> (wasmtime::component::Resource<Bar>, u32) {
                    Host::tuple_result(*self).await
                }
                async fn option_own_arg(
                    &mut self,
                    x: Option<wasmtime::component::Resource<Bar>>,
                ) -> () {
                    Host::option_own_arg(*self, x).await
                }
                async fn option_borrow_arg(
                    &mut self,
                    x: Option<wasmtime::component::Resource<Bar>>,
                ) -> () {
                    Host::option_borrow_arg(*self, x).await
                }
                async fn option_result(
                    &mut self,
                ) -> Option<wasmtime::component::Resource<Bar>> {
                    Host::option_result(*self).await
                }
                async fn result_own_arg(
                    &mut self,
                    x: Result<wasmtime::component::Resource<Bar>, ()>,
                ) -> () {
                    Host::result_own_arg(*self, x).await
                }
                async fn result_borrow_arg(
                    &mut self,
                    x: Result<wasmtime::component::Resource<Bar>, ()>,
                ) -> () {
                    Host::result_borrow_arg(*self, x).await
                }
                async fn result_result(
                    &mut self,
                ) -> Result<wasmtime::component::Resource<Bar>, ()> {
                    Host::result_result(*self).await
                }
                async fn list_own_arg(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::Resource<Bar>,
                    >,
                ) -> () {
                    Host::list_own_arg(*self, x).await
                }
                async fn list_borrow_arg(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::Resource<Bar>,
                    >,
                ) -> () {
                    Host::list_borrow_arg(*self, x).await
                }
                async fn list_result(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::Resource<Bar>,
                > {
                    Host::list_result(*self).await
                }
                async fn record_own_arg(&mut self, x: NestedOwn) -> () {
                    Host::record_own_arg(*self, x).await
                }
                async fn record_borrow_arg(&mut self, x: NestedBorrow) -> () {
                    Host::record_borrow_arg(*self, x).await
                }
                async fn record_result(&mut self) -> NestedOwn {
                    Host::record_result(*self).await
                }
                async fn func_with_handle_typedef(&mut self, x: SomeHandle) -> () {
                    Host::func_with_handle_typedef(*self, x).await
                }
            }
        }
        #[allow(clippy::all)]
        pub mod long_use_chain1 {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub enum A {}
            #[wasmtime::component::__internal::async_trait]
            pub trait HostA {
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<A>,
                ) -> wasmtime::Result<()>;
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: HostA + ?Sized + Send> HostA for &mut _T {
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<A>,
                ) -> wasmtime::Result<()> {
                    HostA::drop(*self, rep).await
                }
            }
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send + HostA {}
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
                let mut inst = linker.instance("foo:foo/long-use-chain1")?;
                inst.resource_async(
                    "a",
                    wasmtime::component::ResourceType::host::<A>(),
                    move |mut store, rep| {
                        std::boxed::Box::new(async move {
                            HostA::drop(
                                    &mut host_getter(store.data_mut()),
                                    wasmtime::component::Resource::new_own(rep),
                                )
                                .await
                        })
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
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
        #[allow(clippy::all)]
        pub mod long_use_chain2 {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type A = super::super::super::foo::foo::long_use_chain1::A;
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {}
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
                let mut inst = linker.instance("foo:foo/long-use-chain2")?;
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
        #[allow(clippy::all)]
        pub mod long_use_chain3 {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type A = super::super::super::foo::foo::long_use_chain2::A;
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {}
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
                let mut inst = linker.instance("foo:foo/long-use-chain3")?;
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
        #[allow(clippy::all)]
        pub mod long_use_chain4 {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type A = super::super::super::foo::foo::long_use_chain3::A;
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send {
                async fn foo(&mut self) -> wasmtime::component::Resource<A>;
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
                let mut inst = linker.instance("foo:foo/long-use-chain4")?;
                inst.func_wrap_async(
                    "foo",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        use tracing::Instrument;
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen import", module =
                            "long-use-chain4", function = "foo",
                        );
                        wasmtime::component::__internal::Box::new(
                            async move {
                                tracing::event!(tracing::Level::TRACE, "call");
                                let host = &mut host_getter(caller.data_mut());
                                let r = Host::foo(host).await;
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
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                async fn foo(&mut self) -> wasmtime::component::Resource<A> {
                    Host::foo(*self).await
                }
            }
        }
        #[allow(clippy::all)]
        pub mod transitive_interface_with_resource {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub enum Foo {}
            #[wasmtime::component::__internal::async_trait]
            pub trait HostFoo {
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Foo>,
                ) -> wasmtime::Result<()>;
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: HostFoo + ?Sized + Send> HostFoo for &mut _T {
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Foo>,
                ) -> wasmtime::Result<()> {
                    HostFoo::drop(*self, rep).await
                }
            }
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send + HostFoo {}
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
                let mut inst = linker
                    .instance("foo:foo/transitive-interface-with-resource")?;
                inst.resource_async(
                    "foo",
                    wasmtime::component::ResourceType::host::<Foo>(),
                    move |mut store, rep| {
                        std::boxed::Box::new(async move {
                            HostFoo::drop(
                                    &mut host_getter(store.data_mut()),
                                    wasmtime::component::Resource::new_own(rep),
                                )
                                .await
                        })
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
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod uses_resource_transitively {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                pub type Foo = super::super::super::super::foo::foo::transitive_interface_with_resource::Foo;
                pub struct Guest {
                    handle: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    handle: wasmtime::component::ComponentExportIndex,
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
                            .export_index(None, "foo:foo/uses-resource-transitively")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/uses-resource-transitively`"
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
                            .get_export(
                                &mut store,
                                None,
                                "foo:foo/uses-resource-transitively",
                            )
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/uses-resource-transitively`"
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
                                        "instance export `foo:foo/uses-resource-transitively` does \
                        not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let handle = lookup("handle")?;
                        Ok(GuestIndices { handle })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let handle = *_instance
                            .get_typed_func::<
                                (wasmtime::component::Resource<Foo>,),
                                (),
                            >(&mut store, &self.handle)?
                            .func();
                        Ok(Guest { handle })
                    }
                }
                impl Guest {
                    pub async fn call_handle<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::Resource<Foo>,
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let span = tracing::span!(
                            tracing::Level::TRACE, "wit-bindgen export", module =
                            "foo:foo/uses-resource-transitively", function = "handle",
                        );
                        let _enter = span.enter();
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::Resource<Foo>,),
                                (),
                            >::new_unchecked(self.handle)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                }
            }
        }
    }
}
