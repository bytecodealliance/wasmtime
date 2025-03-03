/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `imports`.
///
/// This structure is created through [`ImportsPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Imports`] as well.
pub struct ImportsPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: ImportsIndices,
}
impl<T> Clone for ImportsPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> ImportsPre<_T> {
    /// Creates a new copy of `ImportsPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = ImportsIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`Imports`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Imports>
    where
        _T: Send + 'static,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `imports`.
///
/// This is an implementation detail of [`ImportsPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Imports`] as well.
#[derive(Clone)]
pub struct ImportsIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `imports`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Imports::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`ImportsPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`ImportsPre::instantiate_async`] to
///   create a [`Imports`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Imports::new`].
///
/// * You can also access the guts of instantiation through
///   [`ImportsIndices::new_instance`] followed
///   by [`ImportsIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Imports {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl ImportsIndices {
        /// Creates a new copy of `ImportsIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            Ok(ImportsIndices {})
        }
        /// Creates a new instance of [`ImportsIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`Imports`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`Imports`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            Ok(ImportsIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Imports`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Imports> {
            let _instance = instance;
            Ok(Imports {})
        }
    }
    impl Imports {
        /// Convenience wrapper around [`ImportsPre::new`] and
        /// [`ImportsPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Imports>
        where
            _T: Send + 'static,
        {
            let pre = linker.instantiate_pre(component)?;
            ImportsPre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`ImportsIndices::new_instance`] and
        /// [`ImportsIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Imports> {
            let indices = ImportsIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send + a::b::interface_with_live_type::Host<Data = T>
                + a::b::interface_with_dead_type::Host + 'static,
            U: Send + a::b::interface_with_live_type::Host<Data = T>
                + a::b::interface_with_dead_type::Host,
        {
            a::b::interface_with_live_type::add_to_linker(linker, get)?;
            a::b::interface_with_dead_type::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
pub mod a {
    pub mod b {
        #[allow(clippy::all)]
        pub mod interface_with_live_type {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct LiveType {
                #[component(name = "a")]
                pub a: u32,
            }
            impl core::fmt::Debug for LiveType {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("LiveType").field("a", &self.a).finish()
                }
            }
            const _: () = {
                assert!(4 == < LiveType as wasmtime::component::ComponentType >::SIZE32);
                assert!(
                    4 == < LiveType as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub trait Host {
                type Data;
                fn f(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> LiveType + Send + Sync + 'static,
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
                let mut inst = linker.instance("a:b/interface-with-live-type")?;
                inst.func_wrap_concurrent(
                    "f",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = caller;
                        let r = <G::Host as Host>::f(host);
                        Box::pin(async move {
                            let fun = r.await;
                            Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                                let r = fun(caller);
                                Ok((r,))
                            })
                                as Box<
                                    dyn FnOnce(
                                        wasmtime::StoreContextMut<'_, T>,
                                    ) -> wasmtime::Result<(LiveType,)> + Send + Sync,
                                >
                        })
                            as ::core::pin::Pin<
                                Box<
                                    dyn ::core::future::Future<
                                        Output = Box<
                                            dyn FnOnce(
                                                wasmtime::StoreContextMut<'_, T>,
                                            ) -> wasmtime::Result<(LiveType,)> + Send + Sync,
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
                fn f(
                    store: wasmtime::StoreContextMut<'_, Self::Data>,
                ) -> impl ::core::future::Future<
                    Output = impl FnOnce(
                        wasmtime::StoreContextMut<'_, Self::Data>,
                    ) -> LiveType + Send + Sync + 'static,
                > + Send + Sync + 'static
                where
                    Self: Sized,
                {
                    <_T as Host>::f(store)
                }
            }
        }
        #[allow(clippy::all)]
        pub mod interface_with_dead_type {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            pub type LiveType = super::super::super::a::b::interface_with_live_type::LiveType;
            const _: () = {
                assert!(4 == < LiveType as wasmtime::component::ComponentType >::SIZE32);
                assert!(
                    4 == < LiveType as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone, Copy)]
            pub struct DeadType {
                #[component(name = "a")]
                pub a: u32,
            }
            impl core::fmt::Debug for DeadType {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("DeadType").field("a", &self.a).finish()
                }
            }
            const _: () = {
                assert!(4 == < DeadType as wasmtime::component::ComponentType >::SIZE32);
                assert!(
                    4 == < DeadType as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone, Copy)]
            pub enum V {
                #[component(name = "a")]
                A(LiveType),
                #[component(name = "b")]
                B(DeadType),
            }
            impl core::fmt::Debug for V {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        V::A(e) => f.debug_tuple("V::A").field(e).finish(),
                        V::B(e) => f.debug_tuple("V::B").field(e).finish(),
                    }
                }
            }
            const _: () = {
                assert!(8 == < V as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < V as wasmtime::component::ComponentType >::ALIGN32);
            };
            pub trait Host {}
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
                T: Send + 'static,
            {
                let mut inst = linker.instance("a:b/interface-with-dead-type")?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host + Send,
                T: Send + 'static,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {}
        }
    }
}
