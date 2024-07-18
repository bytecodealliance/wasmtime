/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `http-interface`.
///
/// This structure is created through [`HttpInterfacePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct HttpInterfacePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::http_handler::GuestPre,
}
impl<T> Clone for HttpInterfacePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            interface0: self.interface0.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `http-interface`.
///
/// This structure is created through either
/// [`HttpInterface::instantiate_async`] or by first creating
/// a [`HttpInterfacePre`] followed by using
/// [`HttpInterfacePre::instantiate_async`].
pub struct HttpInterface {
    interface0: exports::http_handler::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> HttpInterfacePre<_T> {
        /// Creates a new copy of `HttpInterfacePre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::http_handler::GuestPre::new(_component)?;
            Ok(HttpInterfacePre {
                instance_pre,
                interface0,
            })
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
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(HttpInterface { interface0 })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
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
            use wasmtime::component::__internal::anyhow;
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
            #[wasmtime::component::__internal::async_trait]
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
        }
    }
}
#[allow(clippy::all)]
pub mod http_fetch {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
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
    #[wasmtime::component::__internal::async_trait]
    pub trait Host: Send {
        async fn fetch_request(&mut self, request: Request) -> Response;
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
        let mut inst = linker.instance("http-fetch")?;
        inst.func_wrap_async(
            "fetch-request",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Request,)| wasmtime::component::__internal::Box::new(async move {
                let host = &mut host_getter(caller.data_mut());
                let r = Host::fetch_request(host, arg0).await;
                Ok((r,))
            }),
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
        async fn fetch_request(&mut self, request: Request) -> Response {
            Host::fetch_request(*self, request).await
        }
    }
}
pub mod exports {
    #[allow(clippy::all)]
    pub mod http_handler {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::anyhow;
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
        pub struct GuestPre {
            handle_request: wasmtime::component::ComponentExportIndex,
        }
        impl GuestPre {
            pub fn new(
                component: &wasmtime::component::Component,
            ) -> wasmtime::Result<GuestPre> {
                let _component = component;
                let (_, instance) = component
                    .export_index(None, "http-handler")
                    .ok_or_else(|| {
                        anyhow::anyhow!("no exported instance named `http-handler`")
                    })?;
                let _lookup = |name: &str| {
                    _component
                        .export_index(Some(&instance), name)
                        .map(|p| p.1)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "instance export `http-handler` does \
            not have export `{name}`"
                            )
                        })
                };
                let handle_request = _lookup("handle-request")?;
                Ok(GuestPre { handle_request })
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
                let callee = unsafe {
                    wasmtime::component::TypedFunc::<
                        (&Request,),
                        (Response,),
                    >::new_unchecked(self.handle_request)
                };
                let (ret0,) = callee.call_async(store.as_context_mut(), (arg0,)).await?;
                callee.post_return_async(store.as_context_mut()).await?;
                Ok(ret0)
            }
        }
    }
}
