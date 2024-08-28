/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `neptune`.
///
/// This structure is created through [`NeptunePre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`Neptune`] as well.
pub struct NeptunePre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: NeptuneIndices,
}
impl<T> Clone for NeptunePre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T> NeptunePre<_T> {
    /// Creates a new copy of `NeptunePre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = NeptuneIndices::new(instance_pre.component())?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`Neptune`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<Neptune> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `neptune`.
///
/// This is an implementation detail of [`NeptunePre`] and can
/// be constructed if needed as well.
///
/// For more information see [`Neptune`] as well.
#[derive(Clone)]
pub struct NeptuneIndices {}
/// Auto-generated bindings for an instance a component which
/// implements the world `neptune`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`Neptune::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`NeptunePre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`NeptunePre::instantiate`] to
///   create a [`Neptune`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`Neptune::new`].
///
/// * You can also access the guts of instantiation through
///   [`NeptuneIndices::new_instance`] followed
///   by [`NeptuneIndices::load`] to crate an instance of this
///   type.
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct Neptune {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl NeptuneIndices {
        /// Creates a new copy of `NeptuneIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new(
            component: &wasmtime::component::Component,
        ) -> wasmtime::Result<Self> {
            let _component = component;
            Ok(NeptuneIndices {})
        }
        /// Creates a new instance of [`NeptuneIndices`] from an
        /// instantiated component.
        ///
        /// This method of creating a [`Neptune`] will perform string
        /// lookups for all exports when this method is called. This
        /// will only succeed if the provided instance matches the
        /// requirements of [`Neptune`].
        pub fn new_instance(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let _instance = instance;
            Ok(NeptuneIndices {})
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`Neptune`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Neptune> {
            let _instance = instance;
            Ok(Neptune {})
        }
    }
    impl Neptune {
        /// Convenience wrapper around [`NeptunePre::new`] and
        /// [`NeptunePre::instantiate`].
        pub fn instantiate<_T>(
            mut store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<Neptune> {
            let pre = linker.instantiate_pre(component)?;
            NeptunePre::new(pre)?.instantiate(store)
        }
        /// Convenience wrapper around [`NeptuneIndices::new_instance`] and
        /// [`NeptuneIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Neptune> {
            let indices = NeptuneIndices::new_instance(&mut store, instance)?;
            indices.load(store, instance)
        }
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::green::Host + foo::foo::red::Host,
        {
            foo::foo::green::add_to_linker(linker, get)?;
            foo::foo::red::add_to_linker(linker, get)?;
            Ok(())
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod green {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type Thing = i32;
            const _: () = {
                assert!(4 == < Thing as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Thing as wasmtime::component::ComponentType >::ALIGN32);
            };
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
                let mut inst = linker.instance("foo:foo/green")?;
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
        #[allow(clippy::all)]
        pub mod red {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type Thing = super::super::super::foo::foo::green::Thing;
            const _: () = {
                assert!(4 == < Thing as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Thing as wasmtime::component::ComponentType >::ALIGN32);
            };
            pub trait Host {
                fn foo(&mut self) -> Thing;
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
                let mut inst = linker.instance("foo:foo/red")?;
                inst.func_wrap(
                    "foo",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::foo(host);
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
                fn foo(&mut self) -> Thing {
                    Host::foo(*self)
                }
            }
        }
    }
}
