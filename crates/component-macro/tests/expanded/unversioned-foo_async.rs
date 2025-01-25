/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `nope`.
///
/// This structure is created through [`NopePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Nope`] as well.
pub struct NopePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: NopeIndices,
}
impl<T> Clone for NopePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> NopePre<_T> {
    /// Creates a new copy of `NopePre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = NopeIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`Nope`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Nope>
    where
        _T: Send,
    {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `nope`.
///
/// This is an implementation detail of [`NopePre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Nope`] as well.
#[derive(Clone)]
pub struct NopeIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `nope`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Nope::instantiate_async`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`NopePre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`NopePre::instantiate_async`] to
///   create a [`Nope`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Nope::new`].
///
/// * You can also access the guts of instantiation through
///   [`NopeIndices::new_instance`] followed
///   by [`NopeIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Nope {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl NopeIndices {
        /// Creates a new copy of `NopeIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            Ok(NopeIndices {})
        }
        /// Creates a new instance of [`NopeIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`Nope`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`Nope`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            Ok(NopeIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Nope`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Nope> {
            let _instance = instance;
            Ok(Nope {})
        }
    }
    impl Nope {
        /// Convenience wrapper around [`NopePre::new`] and
        /// [`NopePre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Nope>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            NopePre::new(pre)?.instantiate_async(store).await
        }
        /// Convenience wrapper around [`NopeIndices::new_instance`] and
        /// [`NopeIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Nope> {
            let indices = NopeIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send,
            U: foo::foo::a::Host + Send,
        {
            foo::foo::a::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod a {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::{anyhow, Box};
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum Error {
                #[component(name = "other")]
                Other(wasmtime::component::__internal::String),
            }
            impl core::fmt::Debug for Error {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Error::Other(e) => {
                            f.debug_tuple("Error::Other").field(e).finish()
                        }
                    }
                }
            }
            impl core::fmt::Display for Error {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{:?}", self)
                }
            }
            impl core::error::Error for Error {}
            const _: () = {
                assert!(12 == < Error as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Error as wasmtime::component::ComponentType >::ALIGN32);
            };
            #[wasmtime::component::__internal::trait_variant_make(::core::marker::Send)]
            pub trait Host: Send {
                async fn g(&mut self) -> Result<(), Error>;
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
                let mut inst = linker.instance("foo:foo/a")?;
                inst.func_wrap_async(
                    "g",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        wasmtime::component::__internal::Box::new(async move {
                            let host = &mut host_getter(caller.data_mut());
                            let r = Host::g(host).await;
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
                async fn g(&mut self) -> Result<(), Error> {
                    Host::g(*self).await
                }
            }
        }
    }
}
