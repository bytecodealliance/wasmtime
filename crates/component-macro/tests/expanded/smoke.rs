pub struct TheWorld {}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorld {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: imports::Host,
        {
            imports::add_to_linker(linker, get)?;
            Ok(())
        }
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate(&mut store, component)?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Instantiates a pre-instantiated module using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate_pre<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance_pre: &wasmtime::component::InstancePre<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = instance_pre.instantiate(&mut store)?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Low-level creation wrapper for wrapping up the exports
        /// of the `instance` provided in this structure of wasm
        /// exports.
        ///
        /// This function will extract exports from the `instance`
        /// defined within `store` and wrap them all up in the
        /// returned structure which can be used to interact with
        /// the wasm module.
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<Self> {
            let mut store = store.as_context_mut();
            let mut exports = instance.exports(&mut store);
            let mut __exports = exports.root();
            Ok(TheWorld {})
        }
    }
};
#[allow(clippy::all)]
pub mod imports {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    pub trait Host {
        fn y(&mut self) -> ();
    }
    pub trait GetHost<T>: Send + Sync + Copy + 'static {
        fn get_host<'a>(&self, data: &'a mut T) -> impl Host;
    }
    pub fn add_to_linker_get_host<T>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: impl GetHost<T>,
    ) -> wasmtime::Result<()> {
        let mut inst = linker.instance("imports")?;
        inst.func_wrap(
            "y",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                let host = &mut host_getter.get_host(caller.data_mut());
                let r = Host::y(host);
                Ok(r)
            },
        )?;
        Ok(())
    }
    impl<T, U, F> GetHost<T> for F
    where
        U: Host,
        F: Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    {
        fn get_host<'a>(&self, data: &'a mut T) -> impl Host {
            self(data)
        }
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
        fn y(&mut self) -> () {
            Host::y(*self)
        }
    }
}
