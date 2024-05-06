pub struct TheWorld {
    interface0: exports::foo::foo::manyarg::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheWorld {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::manyarg::Host,
        {
            foo::foo::manyarg::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::manyarg::Guest::new(
                &mut __exports
                    .instance("foo:foo/manyarg")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "exported instance `foo:foo/manyarg` not present"
                        )
                    })?,
            )?;
            Ok(TheWorld { interface0 })
        }
        pub fn foo_foo_manyarg(&self) -> &exports::foo::foo::manyarg::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod manyarg {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct BigStruct {
                #[component(name = "a1")]
                pub a1: wasmtime::component::__internal::String,
                #[component(name = "a2")]
                pub a2: wasmtime::component::__internal::String,
                #[component(name = "a3")]
                pub a3: wasmtime::component::__internal::String,
                #[component(name = "a4")]
                pub a4: wasmtime::component::__internal::String,
                #[component(name = "a5")]
                pub a5: wasmtime::component::__internal::String,
                #[component(name = "a6")]
                pub a6: wasmtime::component::__internal::String,
                #[component(name = "a7")]
                pub a7: wasmtime::component::__internal::String,
                #[component(name = "a8")]
                pub a8: wasmtime::component::__internal::String,
                #[component(name = "a9")]
                pub a9: wasmtime::component::__internal::String,
                #[component(name = "a10")]
                pub a10: wasmtime::component::__internal::String,
                #[component(name = "a11")]
                pub a11: wasmtime::component::__internal::String,
                #[component(name = "a12")]
                pub a12: wasmtime::component::__internal::String,
                #[component(name = "a13")]
                pub a13: wasmtime::component::__internal::String,
                #[component(name = "a14")]
                pub a14: wasmtime::component::__internal::String,
                #[component(name = "a15")]
                pub a15: wasmtime::component::__internal::String,
                #[component(name = "a16")]
                pub a16: wasmtime::component::__internal::String,
                #[component(name = "a17")]
                pub a17: wasmtime::component::__internal::String,
                #[component(name = "a18")]
                pub a18: wasmtime::component::__internal::String,
                #[component(name = "a19")]
                pub a19: wasmtime::component::__internal::String,
                #[component(name = "a20")]
                pub a20: wasmtime::component::__internal::String,
            }
            impl core::fmt::Debug for BigStruct {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("BigStruct")
                        .field("a1", &self.a1)
                        .field("a2", &self.a2)
                        .field("a3", &self.a3)
                        .field("a4", &self.a4)
                        .field("a5", &self.a5)
                        .field("a6", &self.a6)
                        .field("a7", &self.a7)
                        .field("a8", &self.a8)
                        .field("a9", &self.a9)
                        .field("a10", &self.a10)
                        .field("a11", &self.a11)
                        .field("a12", &self.a12)
                        .field("a13", &self.a13)
                        .field("a14", &self.a14)
                        .field("a15", &self.a15)
                        .field("a16", &self.a16)
                        .field("a17", &self.a17)
                        .field("a18", &self.a18)
                        .field("a19", &self.a19)
                        .field("a20", &self.a20)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    160 == < BigStruct as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < BigStruct as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub trait Host {
                fn many_args(
                    &mut self,
                    a1: u64,
                    a2: u64,
                    a3: u64,
                    a4: u64,
                    a5: u64,
                    a6: u64,
                    a7: u64,
                    a8: u64,
                    a9: u64,
                    a10: u64,
                    a11: u64,
                    a12: u64,
                    a13: u64,
                    a14: u64,
                    a15: u64,
                    a16: u64,
                ) -> ();
                fn big_argument(&mut self, x: BigStruct) -> ();
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                U: Host,
            {
                let mut inst = linker.instance("foo:foo/manyarg")?;
                inst.func_wrap(
                    "many-args",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                            arg6,
                            arg7,
                            arg8,
                            arg9,
                            arg10,
                            arg11,
                            arg12,
                            arg13,
                            arg14,
                            arg15,
                        ): (
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                            u64,
                        )|
                    {
                        let host = get(caller.data_mut());
                        let r = Host::many_args(
                            host,
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            arg4,
                            arg5,
                            arg6,
                            arg7,
                            arg8,
                            arg9,
                            arg10,
                            arg11,
                            arg12,
                            arg13,
                            arg14,
                            arg15,
                        );
                        Ok(r)
                    },
                )?;
                inst.func_wrap(
                    "big-argument",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (BigStruct,)|
                    {
                        let host = get(caller.data_mut());
                        let r = Host::big_argument(host, arg0);
                        Ok(r)
                    },
                )?;
                Ok(())
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod manyarg {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct BigStruct {
                    #[component(name = "a1")]
                    pub a1: wasmtime::component::__internal::String,
                    #[component(name = "a2")]
                    pub a2: wasmtime::component::__internal::String,
                    #[component(name = "a3")]
                    pub a3: wasmtime::component::__internal::String,
                    #[component(name = "a4")]
                    pub a4: wasmtime::component::__internal::String,
                    #[component(name = "a5")]
                    pub a5: wasmtime::component::__internal::String,
                    #[component(name = "a6")]
                    pub a6: wasmtime::component::__internal::String,
                    #[component(name = "a7")]
                    pub a7: wasmtime::component::__internal::String,
                    #[component(name = "a8")]
                    pub a8: wasmtime::component::__internal::String,
                    #[component(name = "a9")]
                    pub a9: wasmtime::component::__internal::String,
                    #[component(name = "a10")]
                    pub a10: wasmtime::component::__internal::String,
                    #[component(name = "a11")]
                    pub a11: wasmtime::component::__internal::String,
                    #[component(name = "a12")]
                    pub a12: wasmtime::component::__internal::String,
                    #[component(name = "a13")]
                    pub a13: wasmtime::component::__internal::String,
                    #[component(name = "a14")]
                    pub a14: wasmtime::component::__internal::String,
                    #[component(name = "a15")]
                    pub a15: wasmtime::component::__internal::String,
                    #[component(name = "a16")]
                    pub a16: wasmtime::component::__internal::String,
                    #[component(name = "a17")]
                    pub a17: wasmtime::component::__internal::String,
                    #[component(name = "a18")]
                    pub a18: wasmtime::component::__internal::String,
                    #[component(name = "a19")]
                    pub a19: wasmtime::component::__internal::String,
                    #[component(name = "a20")]
                    pub a20: wasmtime::component::__internal::String,
                }
                impl core::fmt::Debug for BigStruct {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("BigStruct")
                            .field("a1", &self.a1)
                            .field("a2", &self.a2)
                            .field("a3", &self.a3)
                            .field("a4", &self.a4)
                            .field("a5", &self.a5)
                            .field("a6", &self.a6)
                            .field("a7", &self.a7)
                            .field("a8", &self.a8)
                            .field("a9", &self.a9)
                            .field("a10", &self.a10)
                            .field("a11", &self.a11)
                            .field("a12", &self.a12)
                            .field("a13", &self.a13)
                            .field("a14", &self.a14)
                            .field("a15", &self.a15)
                            .field("a16", &self.a16)
                            .field("a17", &self.a17)
                            .field("a18", &self.a18)
                            .field("a19", &self.a19)
                            .field("a20", &self.a20)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        160 == < BigStruct as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < BigStruct as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                pub struct Guest {
                    many_args: wasmtime::component::Func,
                    big_argument: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let many_args = *__exports
                            .typed_func::<
                                (
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                ),
                                (),
                            >("many-args")?
                            .func();
                        let big_argument = *__exports
                            .typed_func::<(&BigStruct,), ()>("big-argument")?
                            .func();
                        Ok(Guest { many_args, big_argument })
                    }
                    pub fn call_many_args<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: u64,
                        arg1: u64,
                        arg2: u64,
                        arg3: u64,
                        arg4: u64,
                        arg5: u64,
                        arg6: u64,
                        arg7: u64,
                        arg8: u64,
                        arg9: u64,
                        arg10: u64,
                        arg11: u64,
                        arg12: u64,
                        arg13: u64,
                        arg14: u64,
                        arg15: u64,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                    u64,
                                ),
                                (),
                            >::new_unchecked(self.many_args)
                        };
                        let () = callee
                            .call(
                                store.as_context_mut(),
                                (
                                    arg0,
                                    arg1,
                                    arg2,
                                    arg3,
                                    arg4,
                                    arg5,
                                    arg6,
                                    arg7,
                                    arg8,
                                    arg9,
                                    arg10,
                                    arg11,
                                    arg12,
                                    arg13,
                                    arg14,
                                    arg15,
                                ),
                            )?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                    pub fn call_big_argument<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &BigStruct,
                    ) -> wasmtime::Result<()> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&BigStruct,),
                                (),
                            >::new_unchecked(self.big_argument)
                        };
                        let () = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(())
                    }
                }
            }
        }
    }
}
