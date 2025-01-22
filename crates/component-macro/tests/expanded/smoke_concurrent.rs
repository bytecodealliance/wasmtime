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
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            T: Send + imports::Host<Data = T> + 'static,
            U: Send + imports::Host<Data = T>,
        {
            imports::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
#[allow(clippy::all)]
pub mod imports {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::{anyhow, Box};
    pub trait Host {
        type Data;
        fn y(
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
        let mut inst = linker.instance("imports")?;
        inst.func_wrap_concurrent(
            "y",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                let host = caller;
                let r = <G::Host as Host>::y(host);
                Box::pin(async move {
                    let fun = r.await;
                    Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
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
        fn y(
            store: wasmtime::StoreContextMut<'_, Self::Data>,
        ) -> impl ::std::future::Future<
            Output = impl FnOnce(
                wasmtime::StoreContextMut<'_, Self::Data>,
            ) -> () + Send + Sync + 'static,
        > + Send + Sync + 'static
        where
            Self: Sized,
        {
            <_T as Host>::y(store)
        }
    }
}
