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
    interface0: exports::foo::foo::anon::GuestIndices,
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
    interface0: exports::foo::foo::anon::Guest,
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
            let interface0 = exports::foo::foo::anon::GuestIndices::new(_component)?;
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
            let interface0 = exports::foo::foo::anon::GuestIndices::new_instance(
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
            U: foo::foo::anon::Host + Send,
        {
            foo::foo::anon::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn foo_foo_anon(&self) -> &exports::foo::foo::anon::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod anon {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(enum)]
            #[derive(Clone, Copy, Eq, PartialEq)]
            #[repr(u8)]
            pub enum Error {
                #[component(name = "success")]
                Success,
                #[component(name = "failure")]
                Failure,
            }
            impl Error {
                pub fn name(&self) -> &'static str {
                    match self {
                        Error::Success => "success",
                        Error::Failure => "failure",
                    }
                }
                pub fn message(&self) -> &'static str {
                    match self {
                        Error::Success => "",
                        Error::Failure => "",
                    }
                }
            }
            impl core::fmt::Debug for Error {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Error")
                        .field("code", &(*self as i32))
                        .field("name", &self.name())
                        .field("message", &self.message())
                        .finish()
                }
            }
            impl core::fmt::Display for Error {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{} (error {})", self.name(), * self as i32)
                }
            }
            impl std::error::Error for Error {}
            const _: () = {
                assert!(1 == < Error as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Error as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
            pub trait Host: Send {
                async fn option_test(
                    &mut self,
                ) -> Result<Option<wasmtime::component::__internal::String>, Error>;
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
                let mut inst = linker.instance("foo:foo/anon")?;
                inst.func_wrap_async(
                    "option-test",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        wasmtime::component::__internal::Box::new(async move {
                            let host = &mut host_getter(caller.data_mut());
                            let r = Host::option_test(host).await;
                            Ok((r,))
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {
                async fn option_test(
                    &mut self,
                ) -> Result<Option<wasmtime::component::__internal::String>, Error> {
                    Host::option_test(*self).await
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod anon {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(enum)]
                #[derive(Clone, Copy, Eq, PartialEq)]
                #[repr(u8)]
                pub enum Error {
                    #[component(name = "success")]
                    Success,
                    #[component(name = "failure")]
                    Failure,
                }
                impl Error {
                    pub fn name(&self) -> &'static str {
                        match self {
                            Error::Success => "success",
                            Error::Failure => "failure",
                        }
                    }
                    pub fn message(&self) -> &'static str {
                        match self {
                            Error::Success => "",
                            Error::Failure => "",
                        }
                    }
                }
                impl core::fmt::Debug for Error {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("Error")
                            .field("code", &(*self as i32))
                            .field("name", &self.name())
                            .field("message", &self.message())
                            .finish()
                    }
                }
                impl core::fmt::Display for Error {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        write!(f, "{} (error {})", self.name(), * self as i32)
                    }
                }
                impl std::error::Error for Error {}
                const _: () = {
                    assert!(
                        1 == < Error as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < Error as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                pub struct Guest {
                    option_test: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    option_test: wasmtime::component::ComponentExportIndex,
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
                            .export_index(None, "foo:foo/anon")
                            .ok_or_else(|| {
                                anyhow::anyhow!("no exported instance named `foo:foo/anon`")
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
                            .get_export(&mut store, None, "foo:foo/anon")
                            .ok_or_else(|| {
                                anyhow::anyhow!("no exported instance named `foo:foo/anon`")
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
                                        "instance export `foo:foo/anon` does \
                    not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let option_test = lookup("option-test")?;
                        Ok(GuestIndices { option_test })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let _instance = instance;
                        let option_test = *_instance
                            .get_typed_func::<
                                (),
                                (
                                    Result<
                                        Option<wasmtime::component::__internal::String>,
                                        Error,
                                    >,
                                ),
                            >(&mut store, &self.option_test)?
                            .func();
                        Ok(Guest { option_test })
                    }
                }
                impl Guest {
                    pub async fn call_option_test<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<
                        Result<Option<wasmtime::component::__internal::String>, Error>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (
                                    Result<
                                        Option<wasmtime::component::__internal::String>,
                                        Error,
                                    >,
                                ),
                            >::new_unchecked(self.option_test)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
