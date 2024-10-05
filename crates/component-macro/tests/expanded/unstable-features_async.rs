pub enum Baz {}
#[wasmtime::component::__internal::async_trait]
pub trait HostBaz {
    async fn foo(&mut self, self_: wasmtime::component::Resource<Baz>) -> ();
    async fn drop(
        &mut self,
        rep: wasmtime::component::Resource<Baz>,
    ) -> wasmtime::Result<()>;
}
#[wasmtime::component::__internal::async_trait]
impl<_T: HostBaz + ?Sized + Send> HostBaz for &mut _T {
    async fn foo(&mut self, self_: wasmtime::component::Resource<Baz>) -> () {
        HostBaz::foo(*self, self_).await
    }
    async fn drop(
        &mut self,
        rep: wasmtime::component::Resource<Baz>,
    ) -> wasmtime::Result<()> {
        HostBaz::drop(*self, rep).await
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
pub struct TheWorldIndices {}
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
pub struct TheWorld {}
#[wasmtime::component::__internal::async_trait]
pub trait TheWorldImports: Send + HostBaz {
    async fn foo(&mut self) -> ();
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
    async fn foo(&mut self) -> () {
        TheWorldImports::foo(*self).await
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
            Ok(TheWorldIndices {})
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
            Ok(TheWorldIndices {})
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
            Ok(TheWorld {})
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
            options: &wasmtime::component::bindgen::LinkOptions,
            host_getter: impl for<'a> TheWorldImportsGetHost<&'a mut T>,
        ) -> wasmtime::Result<()>
        where
            T: Send,
        {
            let mut linker = linker.root();
            if options.has_feature("experimental-world") {
                if options.has_feature("experimental-world-resource") {
                    linker
                        .resource_async(
                            "baz",
                            wasmtime::component::ResourceType::host::<Baz>(),
                            move |mut store, rep| {
                                std::boxed::Box::new(async move {
                                    HostBaz::drop(
                                            &mut host_getter(store.data_mut()),
                                            wasmtime::component::Resource::new_own(rep),
                                        )
                                        .await
                                })
                            },
                        )?;
                }
                if options.has_feature("experimental-world-function-import") {
                    linker
                        .func_wrap_async(
                            "foo",
                            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                                wasmtime::component::__internal::Box::new(async move {
                                    let host = &mut host_getter(caller.data_mut());
                                    let r = TheWorldImports::foo(host).await;
                                    Ok(r)
                                })
                            },
                        )?;
                }
                if options.has_feature("experimental-world-resource-method") {
                    linker
                        .func_wrap_async(
                            "[method]baz.foo",
                            move |
                                mut caller: wasmtime::StoreContextMut<'_, T>,
                                (arg0,): (wasmtime::component::Resource<Baz>,)|
                            {
                                wasmtime::component::__internal::Box::new(async move {
                                    let host = &mut host_getter(caller.data_mut());
                                    let r = HostBaz::foo(host, arg0).await;
                                    Ok(r)
                                })
                            },
                        )?;
                }
            }
            Ok(())
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            options: &wasmtime::component::bindgen::LinkOptions,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::the_interface::Host + TheWorldImports + Send,
        {
            if options.has_feature("experimental-world") {
                Self::add_to_linker_imports_get_host(linker, options, get)?;
                if options.has_feature("experimental-world-interface-import") {
                    foo::foo::the_interface::add_to_linker(linker, options, get)?;
                }
            }
            Ok(())
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod the_interface {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub enum Bar {}
            #[wasmtime::component::__internal::async_trait]
            pub trait HostBar {
                async fn foo(&mut self, self_: wasmtime::component::Resource<Bar>) -> ();
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Bar>,
                ) -> wasmtime::Result<()>;
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: HostBar + ?Sized + Send> HostBar for &mut _T {
                async fn foo(
                    &mut self,
                    self_: wasmtime::component::Resource<Bar>,
                ) -> () {
                    HostBar::foo(*self, self_).await
                }
                async fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Bar>,
                ) -> wasmtime::Result<()> {
                    HostBar::drop(*self, rep).await
                }
            }
            #[wasmtime::component::__internal::async_trait]
            pub trait Host: Send + HostBar {
                async fn foo(&mut self) -> ();
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
                options: &wasmtime::component::bindgen::LinkOptions,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()>
            where
                T: Send,
            {
                if options.has_feature("experimental-interface") {
                    let mut inst = linker.instance("foo:foo/the-interface")?;
                    if options.has_feature("experimental-interface-resource") {
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
                    }
                    if options.has_feature("experimental-interface-function") {
                        inst.func_wrap_async(
                            "foo",
                            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                                wasmtime::component::__internal::Box::new(async move {
                                    let host = &mut host_getter(caller.data_mut());
                                    let r = Host::foo(host).await;
                                    Ok(r)
                                })
                            },
                        )?;
                    }
                    if options.has_feature("experimental-interface-resource-method") {
                        inst.func_wrap_async(
                            "[method]bar.foo",
                            move |
                                mut caller: wasmtime::StoreContextMut<'_, T>,
                                (arg0,): (wasmtime::component::Resource<Bar>,)|
                            {
                                wasmtime::component::__internal::Box::new(async move {
                                    let host = &mut host_getter(caller.data_mut());
                                    let r = HostBar::foo(host, arg0).await;
                                    Ok(r)
                                })
                            },
                        )?;
                    }
                }
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                options: &wasmtime::component::bindgen::LinkOptions,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host + Send,
                T: Send,
            {
                add_to_linker_get_host(linker, options, get)
            }
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                async fn foo(&mut self) -> () {
                    Host::foo(*self).await
                }
            }
        }
    }
}
