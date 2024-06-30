/// Auto-generated bindings for a pre-instantiated version of a
/// copmonent which implements the world `the-world`.
///
/// This structure is created through [`TheWorldPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
pub struct TheWorldPre<T> {
    instance_pre: wasmtime::component::InstancePre<T>,
    interface0: exports::the_name::GuestPre,
}
impl<T> Clone for TheWorldPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            interface0: self.interface0.clone(),
        }
    }
}
/// Auto-generated bindings for an instance a component which
/// implements the world `the-world`.
///
/// This structure is created through either
/// [`TheWorld::instantiate_async`] or by first creating
/// a [`TheWorldPre`] followed by using
/// [`TheWorldPre::instantiate_async`].
pub struct TheWorld {
    interface0: exports::the_name::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl<_T> TheWorldPre<_T> {
        /// Creates a new copy of `TheWorldPre` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the compoennt behind `instance_pre`
        /// does not have the required exports.
        pub fn new(
            instance_pre: wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = instance_pre.component();
            let interface0 = exports::the_name::GuestPre::new(_component)?;
            Ok(TheWorldPre {
                instance_pre,
                interface0,
            })
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
            let _instance = self.instance_pre.instantiate_async(&mut store).await?;
            let interface0 = self.interface0.load(&mut store, &_instance)?;
            Ok(TheWorld { interface0 })
        }
        pub fn engine(&self) -> &wasmtime::Engine {
            self.instance_pre.engine()
        }
        pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
            &self.instance_pre
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
        pub fn the_name(&self) -> &exports::the_name::Guest {
            &self.interface0
        }
    }
};
pub mod exports {
    #[allow(clippy::all)]
    pub mod the_name {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::anyhow;
        pub struct Guest {
            y: wasmtime::component::Func,
        }
        #[derive(Clone)]
        pub struct GuestPre {
            y: wasmtime::component::ComponentExportIndex,
        }
        impl GuestPre {
            pub fn new(
                component: &wasmtime::component::Component,
            ) -> wasmtime::Result<GuestPre> {
                let _component = component;
                let (_, instance) = component
                    .export_index(None, "the-name")
                    .ok_or_else(|| {
                        anyhow::anyhow!("no exported instance named `the-name`")
                    })?;
                let _lookup = |name: &str| {
                    _component
                        .export_index(Some(&instance), name)
                        .map(|p| p.1)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "instance export `the-name` does \
            not have export `{name}`"
                            )
                        })
                };
                let y = _lookup("y")?;
                Ok(GuestPre { y })
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let mut store = store.as_context_mut();
                let _ = &mut store;
                let _instance = instance;
                let y = *_instance.get_typed_func::<(), ()>(&mut store, &self.y)?.func();
                Ok(Guest { y })
            }
        }
        impl Guest {
            pub async fn call_y<S: wasmtime::AsContextMut>(
                &self,
                mut store: S,
            ) -> wasmtime::Result<()>
            where
                <S as wasmtime::AsContext>::Data: Send,
            {
                let callee = unsafe {
                    wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.y)
                };
                let () = callee.call_async(store.as_context_mut(), ()).await?;
                callee.post_return_async(store.as_context_mut()).await?;
                Ok(())
            }
        }
    }
}
