/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`TheWorld`] as well.
pub struct TheWorldPre<T: 'static> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: TheWorldIndices,
}
impl<T: 'static> Clone for TheWorldPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T: 'static> TheWorldPre<_T> {
    /// Creates a new copy of `TheWorldPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = TheWorldIndices::new(&instance_pre)?;
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
    interface0: exports::foo::foo::strings::GuestIndices,
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
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct TheWorld {
    interface0: exports::foo::foo::strings::Guest,
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
        pub fn new<_T>(
            _instance_pre: &wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = _instance_pre.component();
            let _instance_type = _instance_pre.instance_type();
            let interface0 = exports::foo::foo::strings::GuestIndices::new(
                _instance_pre,
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
            let _ = &mut store;
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheWorld { interface0 })
        }
    }
    impl TheWorld {
        /// Convenience wrapper around [`TheWorldPre::new`] and
        /// [`TheWorldPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<TheWorld>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            TheWorldPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`TheWorldIndices::new`] and
        /// [`TheWorldIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<TheWorld> {
            let indices = TheWorldIndices::new(&instance.instance_pre(&store))?;
            indices.load(&mut store, instance)
        }
        pub fn add_to_linker<T, D>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: fn(&mut T) -> D::Data<'_>,
        ) -> wasmtime::Result<()>
        where
            D: foo::foo::strings::HostConcurrent + Send,
            for<'a> D::Data<'a>: foo::foo::strings::Host + Send,
            T: 'static + Send,
        {
            foo::foo::strings::add_to_linker::<T, D>(linker, host_getter)?;
            Ok(())
        }
        pub fn foo_foo_strings(&self) -> &exports::foo::foo::strings::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod strings {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
            pub trait HostConcurrent: wasmtime::component::HasData + Send {
                fn a<T: 'static>(
                    accessor: &mut wasmtime::component::Accessor<T, Self>,
                    x: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<Output = ()> + Send
                where
                    Self: Sized;
                fn b<T: 'static>(
                    accessor: &mut wasmtime::component::Accessor<T, Self>,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::__internal::String,
                > + Send
                where
                    Self: Sized;
                fn c<T: 'static>(
                    accessor: &mut wasmtime::component::Accessor<T, Self>,
                    a: wasmtime::component::__internal::String,
                    b: wasmtime::component::__internal::String,
                ) -> impl ::core::future::Future<
                    Output = wasmtime::component::__internal::String,
                > + Send
                where
                    Self: Sized;
            }
            #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
            pub fn add_to_linker<T, D>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostConcurrent,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                let mut inst = linker.instance("foo:foo/strings")?;
                inst.func_wrap_concurrent(
                    "a",
                    move |
                        caller: &mut wasmtime::component::Accessor<T>,
                        (arg0,): (wasmtime::component::__internal::String,)|
                    {
                        wasmtime::component::__internal::Box::pin(async move {
                            let accessor = &mut unsafe { caller.with_data(host_getter) };
                            let r = <D as HostConcurrent>::a(accessor, arg0).await;
                            Ok(r)
                        })
                    },
                )?;
                inst.func_wrap_concurrent(
                    "b",
                    move |caller: &mut wasmtime::component::Accessor<T>, (): ()| {
                        wasmtime::component::__internal::Box::pin(async move {
                            let accessor = &mut unsafe { caller.with_data(host_getter) };
                            let r = <D as HostConcurrent>::b(accessor).await;
                            Ok((r,))
                        })
                    },
                )?;
                inst.func_wrap_concurrent(
                    "c",
                    move |
                        caller: &mut wasmtime::component::Accessor<T>,
                        (
                            arg0,
                            arg1,
                        ): (
                            wasmtime::component::__internal::String,
                            wasmtime::component::__internal::String,
                        )|
                    {
                        wasmtime::component::__internal::Box::pin(async move {
                            let accessor = &mut unsafe { caller.with_data(host_getter) };
                            let r = <D as HostConcurrent>::c(accessor, arg0, arg1).await;
                            Ok((r,))
                        })
                    },
                )?;
                Ok(())
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod strings {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::{anyhow, Box};
                pub struct Guest {
                    a: wasmtime::component::Func,
                    b: wasmtime::component::Func,
                    c: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    a: wasmtime::component::ComponentExportIndex,
                    b: wasmtime::component::ComponentExportIndex,
                    c: wasmtime::component::ComponentExportIndex,
                }
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new<_T>(
                        _instance_pre: &wasmtime::component::InstancePre<_T>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance = _instance_pre
                            .component()
                            .get_export_index(None, "foo:foo/strings")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "no exported instance named `foo:foo/strings`"
                                )
                            })?;
                        let mut lookup = move |name| {
                            _instance_pre
                                .component()
                                .get_export_index(Some(&instance), name)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "instance export `foo:foo/strings` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let a = lookup("a")?;
                        let b = lookup("b")?;
                        let c = lookup("c")?;
                        Ok(GuestIndices { a, b, c })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let _instance = instance;
                        let _instance_pre = _instance.instance_pre(&store);
                        let _instance_type = _instance_pre.instance_type();
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let a = *_instance
                            .get_typed_func::<(&str,), ()>(&mut store, &self.a)?
                            .func();
                        let b = *_instance
                            .get_typed_func::<
                                (),
                                (wasmtime::component::__internal::String,),
                            >(&mut store, &self.b)?
                            .func();
                        let c = *_instance
                            .get_typed_func::<
                                (&str, &str),
                                (wasmtime::component::__internal::String,),
                            >(&mut store, &self.c)?
                            .func();
                        Ok(Guest { a, b, c })
                    }
                }
                impl Guest {
                    pub fn call_a<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::__internal::String,
                    ) -> impl wasmtime::component::__internal::Future<
                        Output = wasmtime::Result<()>,
                    > + Send + 'static + use<S>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (wasmtime::component::__internal::String,),
                                (),
                            >::new_unchecked(self.a)
                        };
                        callee.call_concurrent(store.as_context_mut(), (arg0,))
                    }
                    pub fn call_b<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> impl wasmtime::component::__internal::Future<
                        Output = wasmtime::Result<
                            wasmtime::component::__internal::String,
                        >,
                    > + Send + 'static + use<S>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::String,),
                            >::new_unchecked(self.b)
                        };
                        wasmtime::component::__internal::FutureExt::map(
                            callee.call_concurrent(store.as_context_mut(), ()),
                            |v| v.map(|(v,)| v),
                        )
                    }
                    pub fn call_c<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: wasmtime::component::__internal::String,
                        arg1: wasmtime::component::__internal::String,
                    ) -> impl wasmtime::component::__internal::Future<
                        Output = wasmtime::Result<
                            wasmtime::component::__internal::String,
                        >,
                    > + Send + 'static + use<S>
                    where
                        <S as wasmtime::AsContext>::Data: Send + 'static,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    wasmtime::component::__internal::String,
                                    wasmtime::component::__internal::String,
                                ),
                                (wasmtime::component::__internal::String,),
                            >::new_unchecked(self.c)
                        };
                        wasmtime::component::__internal::FutureExt::map(
                            callee.call_concurrent(store.as_context_mut(), (arg0, arg1)),
                            |v| v.map(|(v,)| v),
                        )
                    }
                }
            }
        }
    }
}
