pub type U = foo::foo::i::T;
const _: () = {
    assert!(2 == < U as wasmtime::component::ComponentType >::SIZE32);
    assert!(2 == < U as wasmtime::component::ComponentType >::ALIGN32);
};
pub type T = u32;
const _: () = {
    assert!(4 == < T as wasmtime::component::ComponentType >::SIZE32);
    assert!(4 == < T as wasmtime::component::ComponentType >::ALIGN32);
};
#[derive(wasmtime::component::ComponentType)]
#[derive(wasmtime::component::Lift)]
#[derive(wasmtime::component::Lower)]
#[component(record)]
#[derive(Clone, Copy)]
pub struct R {}
impl core::fmt::Debug for R {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("R").finish()
    }
}
const _: () = {
    assert!(0 == < R as wasmtime::component::ComponentType >::SIZE32);
    assert!(1 == < R as wasmtime::component::ComponentType >::ALIGN32);
};
pub struct Foo {
    f: wasmtime::component::Func,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl Foo {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::i::Host,
        {
            foo::foo::i::add_to_linker(linker, get)?;
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
            let f = *__exports.typed_func::<(), ((T, U, R),)>("f")?.func();
            Ok(Foo { f })
        }
        pub fn call_f<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
        ) -> wasmtime::Result<(T, U, R)> {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(), ((T, U, R),)>::new_unchecked(self.f)
            };
            let (ret0,) = callee.call(store.as_context_mut(), ())?;
            callee.post_return(store.as_context_mut())?;
            Ok(ret0)
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod i {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            pub type T = u16;
            const _: () = {
                assert!(2 == < T as wasmtime::component::ComponentType >::SIZE32);
                assert!(2 == < T as wasmtime::component::ComponentType >::ALIGN32);
            };
            pub trait Host {}
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                let mut inst = linker.instance("foo:foo/i")?;
                Ok(())
            }
        }
    }
}
