/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `http-interface`.
///
/// This structure is created through [`HttpInterfacePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`HttpInterface`] as well.
pub struct HttpInterfacePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: HttpInterfaceIndices,
}
impl<T> Clone for HttpInterfacePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> HttpInterfacePre<_T> {
    /// Creates a new copy of `HttpInterfacePre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = HttpInterfaceIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`HttpInterface`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<HttpInterface>
    where
        _T: Send,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `http-interface`.
///
/// This is an implementation detail of [`HttpInterfacePre`] and can
/// be constructed if needed as well.
///
/// For more information see [`HttpInterface`] as well.
#[derive(Clone)]
pub struct HttpInterfaceIndices {
    interface0: exports::http_handler::GuestIndices,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `http-interface`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`HttpInterface::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`HttpInterfacePre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`HttpInterfacePre::instantiate_async`] to
///   create a [`HttpInterface`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`HttpInterface::new`].
///
/// * You can also access the guts of instantiation through
///   [`HttpInterfaceIndices::new_instance`] followed
///   by [`HttpInterfaceIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct HttpInterface {
    interface0: exports::http_handler::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl HttpInterfaceIndices {
        /// Creates a new copy of `HttpInterfaceIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            let interface0 = exports::http_handler::GuestIndices::new(_component)?;
            Ok(HttpInterfaceIndices { interface0 })
        }
        /// Creates a new instance of [`HttpInterfaceIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`HttpInterface`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`HttpInterface`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            let interface0 = exports::http_handler::GuestIndices::new_instance(
                &mut store,
                _instance,
            )?;
            Ok(HttpInterfaceIndices { interface0 })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`HttpInterface`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<HttpInterface> {
            let _instance = instance;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(HttpInterface { interface0 })
        }
    }
    impl HttpInterface {
        /// Convenience wrapper around [`HttpInterfacePre::new`] and
        /// [`HttpInterfacePre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<HttpInterface>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            HttpInterfacePre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`HttpInterfaceIndices::new_instance`] and
        /// [`HttpInterfaceIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<HttpInterface> {
            let indices = HttpInterfaceIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::http_types::Host + http_fetch::Host + Send,
        {
            foo::foo::http_types::add_to_linker(linker, get)?;
            http_fetch::add_to_linker(linker, get)?;
            Ok(())
        }
        pub fn http_handler(&self) -> &exports::http_handler::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod http_types {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct Request {
                #[component(name = "method")]
                pub method: wasmtime::component::__internal::String,
            }
            impl core::fmt::Debug for Request {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Request").field("method", &self.method).finish()
                }
            }
            const _: () = {
                assert!(8 == < Request as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Request as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct Response {
                #[component(name = "body")]
                pub body: wasmtime::component::__internal::String,
            }
            impl core::fmt::Debug for Response {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("Response").field("body", &self.body).finish()
                }
            }
            const _: () = {
                assert!(8 == < Response as wasmtime::component::ComponentType >::SIZE32);
                assert!(
                    4 == < Response as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
            pub trait Host: Send {}
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
                let mut inst = linker.instance("foo:foo/http-types")?;
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
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
    }
}
#[allow(clippy::all)]
pub mod http_fetch {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::{anyhow, Box};
    pub type Request = super::foo::foo::http_types::Request;
    const _: () = {
        assert!(8 == < Request as wasmtime::component::ComponentType >::SIZE32);
        assert!(4 == < Request as wasmtime::component::ComponentType >::ALIGN32);
    };
    pub type Response = super::foo::foo::http_types::Response;
    const _: () = {
        assert!(8 == < Response as wasmtime::component::ComponentType >::SIZE32);
        assert!(4 == < Response as wasmtime::component::ComponentType >::ALIGN32);
    };
    #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
    pub trait Host: Send {
        async fn fetch_request(&mut self, request: Request) -> Response;
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
        let mut inst = linker.instance("http-fetch")?;
        inst.func_wrap_async(
            "fetch-request",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Request,)| {
                use tracing::Instrument;
                let span = tracing::span!(
                    tracing::Level::TRACE, "wit-bindgen import", module = "<no module>",
                    function = "fetch-request",
                );
                wasmtime::component::__internal::Box::new(
                    async move {
                        tracing::event!(
                            tracing::Level::TRACE, request = tracing::field::debug(&
                            arg0), "call"
                        );
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::fetch_request(host, arg0).await;
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
        async fn fetch_request(&mut self, request: Request) -> Response {
            Host::fetch_request(*self, request).await
        }
    }
}
pub mod exports {
    #[allow(clippy::all)]
    pub mod http_handler {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::{anyhow, Box};
        pub type Request = super::super::foo::foo::http_types::Request;
        const _: () = {
            assert!(8 == < Request as wasmtime::component::ComponentType >::SIZE32);
            assert!(4 == < Request as wasmtime::component::ComponentType >::ALIGN32);
        };
        pub type Response = super::super::foo::foo::http_types::Response;
        const _: () = {
            assert!(8 == < Response as wasmtime::component::ComponentType >::SIZE32);
            assert!(4 == < Response as wasmtime::component::ComponentType >::ALIGN32);
        };
        pub struct Guest {
            handle_request: wasmtime::component::Func,
        }
        #[derive(Clone)]
        pub struct GuestIndices {
            handle_request: wasmtime::component::ComponentExportIndex,
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
                    .export_index(None, "http-handler")
                    .ok_or_else(|| {
                        anyhow::anyhow!("no exported instance named `http-handler`")
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
                    .get_export(&mut store, None, "http-handler")
                    .ok_or_else(|| {
                        anyhow::anyhow!("no exported instance named `http-handler`")
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
                                "instance export `http-handler` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                let handle_request = lookup("handle-request")?;
                Ok(GuestIndices { handle_request })
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let mut store = store.as_context_mut();
                let _ = &mut store;
                let _instance = instance;
                let handle_request = *_instance
                    .get_typed_func::<
                        (&Request,),
                        (Response,),
                    >(&mut store, &self.handle_request)?
                    .func();
                Ok(Guest { handle_request })
            }
        }
        impl Guest {
            pub async fn call_handle_request<S: wasmtime::AsContextMut>(
                &self,
                mut store: S,
                arg0: &Request,
            ) -> wasmtime::Result<Response>
            where
                <S as wasmtime::AsContext>::Data: Send,
            {
                use tracing::Instrument;
                let span = tracing::span!(
                    tracing::Level::TRACE, "wit-bindgen export", module = "http-handler",
                    function = "handle-request",
                );
                let callee = unsafe {
                    wasmtime::component::TypedFunc::<
                        (&Request,),
                        (Response,),
                    >::new_unchecked(self.handle_request)
                };
                let (ret0,) = callee
                    .call_async(store.as_context_mut(), (arg0,))
                    .instrument(span.clone())
                    .await?;
                callee.post_return_async(store.as_context_mut()).instrument(span).await?;
                Ok(ret0)
            }
        }
    }
}
