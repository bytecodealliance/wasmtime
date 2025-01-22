/// Link-time configurations.
#[derive(Clone, Debug, Default)]
pub struct LinkOptions {
    experimental_interface: bool,
    experimental_interface_function: bool,
    experimental_interface_resource: bool,
    experimental_interface_resource_method: bool,
    experimental_world: bool,
    experimental_world_function_import: bool,
    experimental_world_interface_import: bool,
    experimental_world_resource: bool,
    experimental_world_resource_method: bool,
}
impl LinkOptions {
    /// Enable members marked as `@unstable(feature = experimental-interface)`
    pub fn experimental_interface(&mut self, enabled: bool) -> &mut Self {
        self.experimental_interface = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-interface-function)`
    pub fn experimental_interface_function(&mut self, enabled: bool) -> &mut Self {
        self.experimental_interface_function = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-interface-resource)`
    pub fn experimental_interface_resource(&mut self, enabled: bool) -> &mut Self {
        self.experimental_interface_resource = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-interface-resource-method)`
    pub fn experimental_interface_resource_method(
        &mut self,
        enabled: bool,
    ) -> &mut Self {
        self.experimental_interface_resource_method = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-world)`
    pub fn experimental_world(&mut self, enabled: bool) -> &mut Self {
        self.experimental_world = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-world-function-import)`
    pub fn experimental_world_function_import(&mut self, enabled: bool) -> &mut Self {
        self.experimental_world_function_import = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-world-interface-import)`
    pub fn experimental_world_interface_import(&mut self, enabled: bool) -> &mut Self {
        self.experimental_world_interface_import = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-world-resource)`
    pub fn experimental_world_resource(&mut self, enabled: bool) -> &mut Self {
        self.experimental_world_resource = enabled;
        self
    }
    /// Enable members marked as `@unstable(feature = experimental-world-resource-method)`
    pub fn experimental_world_resource_method(&mut self, enabled: bool) -> &mut Self {
        self.experimental_world_resource_method = enabled;
        self
    }
}
pub enum Baz {}
pub trait HostBaz: Sized {
    type BazData;
    fn foo(
        store: wasmtime::StoreContextMut<'_, Self::BazData>,
        self_: wasmtime::component::Resource<Baz>,
    ) -> impl ::std::future::Future<
        Output = impl FnOnce(
            wasmtime::StoreContextMut<'_, Self::BazData>,
        ) -> () + Send + Sync + 'static,
    > + Send + Sync + 'static
    where
        Self: Sized;
    fn drop(&mut self, rep: wasmtime::component::Resource<Baz>) -> wasmtime::Result<()>;
}
impl<_T: HostBaz> HostBaz for &mut _T {
    type BazData = _T::BazData;
    fn foo(
        store: wasmtime::StoreContextMut<'_, Self::BazData>,
        self_: wasmtime::component::Resource<Baz>,
    ) -> impl ::std::future::Future<
        Output = impl FnOnce(
            wasmtime::StoreContextMut<'_, Self::BazData>,
        ) -> () + Send + Sync + 'static,
    > + Send + Sync + 'static
    where
        Self: Sized,
    {
        <_T as HostBaz>::foo(store, self_)
    }
    fn drop(&mut self, rep: wasmtime::component::Resource<Baz>) -> wasmtime::Result<()> {
        HostBaz::drop(*self, rep)
    }
}
impl std::convert::From<LinkOptions> for foo::foo::the_interface::LinkOptions {
    fn from(src: LinkOptions) -> Self {
        (&src).into()
    }
}
impl std::convert::From<&LinkOptions> for foo::foo::the_interface::LinkOptions {
    fn from(src: &LinkOptions) -> Self {
        let mut dest = Self::default();
        dest.experimental_interface(src.experimental_interface);
        dest.experimental_interface_function(src.experimental_interface_function);
        dest.experimental_interface_resource(src.experimental_interface_resource);
        dest.experimental_interface_resource_method(
            src.experimental_interface_resource_method,
        );
        dest
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
pub trait TheWorldImports: HostBaz {
    type Data;
    fn foo(
        store: wasmtime::StoreContextMut<'_, Self::Data>,
    ) -> impl ::std::future::Future<
        Output = impl FnOnce(
            wasmtime::StoreContextMut<'_, Self::Data>,
        ) -> () + Send + Sync + 'static,
    > + Send + Sync + 'static
    where
        Self: Sized;
}
pub trait TheWorldImportsGetHost<
    T,
    D,
>: Fn(T) -> <Self as TheWorldImportsGetHost<T, D>>::Host + Send + Sync + Copy + 'static {
    type Host: TheWorldImports<BazData = D, Data = D>;
}
impl<F, T, D, O> TheWorldImportsGetHost<T, D> for F
where
    F: Fn(T) -> O + Send + Sync + Copy + 'static,
    O: TheWorldImports<BazData = D, Data = D>,
{
    type Host = O;
}
impl<_T: TheWorldImports> TheWorldImports for &mut _T {
    type Data = _T::Data;
    fn foo(
        store: wasmtime::StoreContextMut<'_, Self::Data>,
    ) -> impl ::std::future::Future<
        Output = impl FnOnce(
            wasmtime::StoreContextMut<'_, Self::Data>,
        ) -> () + Send + Sync + 'static,
    > + Send + Sync + 'static
    where
        Self: Sized,
    {
        <_T as TheWorldImports>::foo(store)
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
        pub fn add_to_linker_imports_get_host<
            T,
            G: for<'a> TheWorldImportsGetHost<
                    &'a mut T,
                    T,
                    Host: TheWorldImports<BazData = T, Data = T>,
                >,
        >(
            linker: &mut wasmtime::component::Linker<T>,
            options: &LinkOptions,
            host_getter: G,
        ) -> wasmtime::Result<()>
        where
            T: Send + 'static,
        {
            let mut linker = linker.root();
            if options.experimental_world {
                if options.experimental_world_resource {
                    linker
                        .resource(
                            "baz",
                            wasmtime::component::ResourceType::host::<Baz>(),
                            move |mut store, rep| -> wasmtime::Result<()> {
                                HostBaz::drop(
                                    &mut host_getter(store.data_mut()),
                                    wasmtime::component::Resource::new_own(rep),
                                )
                            },
                        )?;
                }
                if options.experimental_world_function_import {
                    linker
                        .func_wrap_concurrent(
                            "foo",
                            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                                let host = caller;
                                let r = <G::Host as TheWorldImports>::foo(host);
                                Box::pin(async move {
                                    let fun = r.await;
                                    Box::new(move |
                                        mut caller: wasmtime::StoreContextMut<'_, T>|
                                    {
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
                }
                if options.experimental_world_resource_method {
                    linker
                        .func_wrap_concurrent(
                            "[method]baz.foo",
                            move |
                                mut caller: wasmtime::StoreContextMut<'_, T>,
                                (arg0,): (wasmtime::component::Resource<Baz>,)|
                            {
                                let host = caller;
                                let r = <G::Host as HostBaz>::foo(host, arg0);
                                Box::pin(async move {
                                    let fun = r.await;
                                    Box::new(move |
                                        mut caller: wasmtime::StoreContextMut<'_, T>|
                                    {
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
                }
            }
            Ok(())
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            options: &LinkOptions,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send + foo::foo::the_interface::Host<BarData = T, Data = T>
                + TheWorldImports<BazData = T, Data = T> + 'static,
            U: Send + foo::foo::the_interface::Host<BarData = T, Data = T>
                + TheWorldImports<BazData = T, Data = T>,
        {
            if options.experimental_world {
                Self::add_to_linker_imports_get_host(linker, options, get)?;
                if options.experimental_world_interface_import {
                    foo::foo::the_interface::add_to_linker(
                        linker,
                        &options.into(),
                        get,
                    )?;
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
            use wasmtime::component::__internal::{anyhow, Box};
            /// Link-time configurations.
            #[derive(Clone, Debug, Default)]
            pub struct LinkOptions {
                experimental_interface: bool,
                experimental_interface_function: bool,
                experimental_interface_resource: bool,
                experimental_interface_resource_method: bool,
            }
            impl LinkOptions {
                /// Enable members marked as `@unstable(feature = experimental-interface)`
                pub fn experimental_interface(&mut self, enabled: bool) -> &mut Self {
                    self.experimental_interface = enabled;
                    self
                }
                /// Enable members marked as `@unstable(feature = experimental-interface-function)`
                pub fn experimental_interface_function(
                    &mut self,
                    enabled: bool,
                ) -> &mut Self {
                    self.experimental_interface_function = enabled;
                    self
                }
                /// Enable members marked as `@unstable(feature = experimental-interface-resource)`
                pub fn experimental_interface_resource(
                    &mut self,
                    enabled: bool,
                ) -> &mut Self {
                    self.experimental_interface_resource = enabled;
                    self
                }
                /// Enable members marked as `@unstable(feature = experimental-interface-resource-method)`
                pub fn experimental_interface_resource_method(
                    &mut self,
                    enabled: bool,
                ) -> &mut Self {
                    self.experimental_interface_resource_method = enabled;
                    self
                }
            }
            pub enum Bar {}
            pub trait HostBar: Sized {
                type BarData;
                fn foo(
                    store: wasmtime::StoreContextMut<'_, Self::BarData>,
                    self_: wasmtime::component::Resource<Bar>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::BarData>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized;
                fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Bar>,
                ) -> wasmtime::Result<()>;
            }
            impl<_T: HostBar> HostBar for &mut _T {
                type BarData = _T::BarData;
                fn foo(
                    store: wasmtime::StoreContextMut<'_, Self::BarData>,
                    self_: wasmtime::component::Resource<Bar>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::BarData>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as HostBar>::foo(store, self_)
                }
                fn drop(
                    &mut self,
                    rep: wasmtime::component::Resource<Bar>,
                ) -> wasmtime::Result<()> {
                    HostBar::drop(*self, rep)
                }
            }
            pub trait Host: HostBar + Sized {
                type Data;
                fn foo(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
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
                type Host: Host<BarData = D, Data = D> + Send;
            }
            impl<F, T, D, O> GetHost<T, D> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host<BarData = D, Data = D> + Send,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<
                T,
                G: for<'a> GetHost<
                        &'a mut T,
                        T,
                        Host: Host<BarData = T, Data = T> + Send,
                    >,
            >(
                linker: &mut wasmtime::component::Linker<T>,
                options: &LinkOptions,
                host_getter: G,
            ) -> wasmtime::Result<()>
            where
                T: Send + 'static,
            {
                if options.experimental_interface {
                    let mut inst = linker.instance("foo:foo/the-interface")?;
                    if options.experimental_interface_resource {
                        inst.resource(
                            "bar",
                            wasmtime::component::ResourceType::host::<Bar>(),
                            move |mut store, rep| -> wasmtime::Result<()> {
                                HostBar::drop(
                                    &mut host_getter(store.data_mut()),
                                    wasmtime::component::Resource::new_own(rep),
                                )
                            },
                        )?;
                    }
                    if options.experimental_interface_function {
                        inst.func_wrap_concurrent(
                            "foo",
                            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                                let host = caller;
                                let r = <G::Host as Host>::foo(host);
                                Box::pin(async move {
                                    let fun = r.await;
                                    Box::new(move |
                                        mut caller: wasmtime::StoreContextMut<'_, T>|
                                    {
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
                    }
                    if options.experimental_interface_resource_method {
                        inst.func_wrap_concurrent(
                            "[method]bar.foo",
                            move |
                                mut caller: wasmtime::StoreContextMut<'_, T>,
                                (arg0,): (wasmtime::component::Resource<Bar>,)|
                            {
                                let host = caller;
                                let r = <G::Host as HostBar>::foo(host, arg0);
                                Box::pin(async move {
                                    let fun = r.await;
                                    Box::new(move |
                                        mut caller: wasmtime::StoreContextMut<'_, T>|
                                    {
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
                    }
                }
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                options: &LinkOptions,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host<BarData = T, Data = T> + Send,
                T: Send + 'static,
            {
                add_to_linker_get_host(linker, options, get)
            }
            impl<_T: Host> Host for &mut _T {
                type Data = _T::Data;
                fn foo(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::std::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> () + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::foo(store)
                }
            }
        }
    }
}
