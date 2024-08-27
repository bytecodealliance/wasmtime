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
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Imports> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
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
///   [`Imports::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`ImportsPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`ImportsPre::instantiate`] to
///   create a [`Imports`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Imports::new_instance`]
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
        /// [`ImportsPre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Imports> {
            let pre = linker.instantiate_pre(component)?;
            ImportsPre::new(pre)?.instantiate(store)
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
            U: a::b::interface_with_live_type::Host
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
            use wasmtime::component::__internal::anyhow;
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
                fn f(&mut self) -> LiveType;
            }
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Host + Send + Sync + Copy + 'static {
                type Host: Host;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("a:b/interface-with-live-type")?;
                inst.func_wrap(
                    "f",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::f(host);
                        Ok((r,))
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {
                fn f(&mut self) -> LiveType {
                    Host::f(*self)
                }
            }
        }
        #[allow(clippy::all)]
        pub mod interface_with_dead_type {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub trait Host {}
            pub trait GetHost<
                T,
            >: Fn(T) -> <Self as GetHost<T>>::Host + Send + Sync + Copy + 'static {
                type Host: Host;
            }
            impl<F, T, O> GetHost<T> for F
            where
                F: Fn(T) -> O + Send + Sync + Copy + 'static,
                O: Host,
            {
                type Host = O;
            }
            pub fn add_to_linker_get_host<T>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: impl for<'a> GetHost<&'a mut T>,
            ) -> wasmtime::Result<()> {
                let mut inst = linker.instance("a:b/interface-with-dead-type")?;
                Ok(())
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                add_to_linker_get_host(linker, get)
            }
            impl<_T: Host + ?Sized> Host for &mut _T {}
        }
    }
}
