pub struct TheFlags {
    interface0: exports::foo::foo::flegs::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheFlags {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::flegs::Host,
        {
            foo::foo::flegs::add_to_linker(linker, get)?;
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
            let interface0 = exports::foo::foo::flegs::Guest::new(
                &mut __exports
                    .instance("foo:foo/flegs")
                    .ok_or_else(|| {
                        anyhow::anyhow!("exported instance `foo:foo/flegs` not present")
                    })?,
            )?;
            Ok(TheFlags { interface0 })
        }
        pub fn foo_foo_flegs(&self) -> &exports::foo::foo::flegs::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod flegs {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            wasmtime::component::flags!(Flag1 { #[component(name = "b0")] const B0; });
            const _: () = {
                assert!(1 == < Flag1 as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Flag1 as wasmtime::component::ComponentType >::ALIGN32);
            };
            wasmtime::component::flags!(
                Flag2 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                const B1; }
            );
            const _: () = {
                assert!(1 == < Flag2 as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Flag2 as wasmtime::component::ComponentType >::ALIGN32);
            };
            wasmtime::component::flags!(
                Flag4 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                const B1; #[component(name = "b2")] const B2; #[component(name = "b3")]
                const B3; }
            );
            const _: () = {
                assert!(1 == < Flag4 as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Flag4 as wasmtime::component::ComponentType >::ALIGN32);
            };
            wasmtime::component::flags!(
                Flag8 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                const B1; #[component(name = "b2")] const B2; #[component(name = "b3")]
                const B3; #[component(name = "b4")] const B4; #[component(name = "b5")]
                const B5; #[component(name = "b6")] const B6; #[component(name = "b7")]
                const B7; }
            );
            const _: () = {
                assert!(1 == < Flag8 as wasmtime::component::ComponentType >::SIZE32);
                assert!(1 == < Flag8 as wasmtime::component::ComponentType >::ALIGN32);
            };
            wasmtime::component::flags!(
                Flag16 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                const B1; #[component(name = "b2")] const B2; #[component(name = "b3")]
                const B3; #[component(name = "b4")] const B4; #[component(name = "b5")]
                const B5; #[component(name = "b6")] const B6; #[component(name = "b7")]
                const B7; #[component(name = "b8")] const B8; #[component(name = "b9")]
                const B9; #[component(name = "b10")] const B10; #[component(name =
                "b11")] const B11; #[component(name = "b12")] const B12; #[component(name
                = "b13")] const B13; #[component(name = "b14")] const B14;
                #[component(name = "b15")] const B15; }
            );
            const _: () = {
                assert!(2 == < Flag16 as wasmtime::component::ComponentType >::SIZE32);
                assert!(2 == < Flag16 as wasmtime::component::ComponentType >::ALIGN32);
            };
            wasmtime::component::flags!(
                Flag32 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                const B1; #[component(name = "b2")] const B2; #[component(name = "b3")]
                const B3; #[component(name = "b4")] const B4; #[component(name = "b5")]
                const B5; #[component(name = "b6")] const B6; #[component(name = "b7")]
                const B7; #[component(name = "b8")] const B8; #[component(name = "b9")]
                const B9; #[component(name = "b10")] const B10; #[component(name =
                "b11")] const B11; #[component(name = "b12")] const B12; #[component(name
                = "b13")] const B13; #[component(name = "b14")] const B14;
                #[component(name = "b15")] const B15; #[component(name = "b16")] const
                B16; #[component(name = "b17")] const B17; #[component(name = "b18")]
                const B18; #[component(name = "b19")] const B19; #[component(name =
                "b20")] const B20; #[component(name = "b21")] const B21; #[component(name
                = "b22")] const B22; #[component(name = "b23")] const B23;
                #[component(name = "b24")] const B24; #[component(name = "b25")] const
                B25; #[component(name = "b26")] const B26; #[component(name = "b27")]
                const B27; #[component(name = "b28")] const B28; #[component(name =
                "b29")] const B29; #[component(name = "b30")] const B30; #[component(name
                = "b31")] const B31; }
            );
            const _: () = {
                assert!(4 == < Flag32 as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Flag32 as wasmtime::component::ComponentType >::ALIGN32);
            };
            wasmtime::component::flags!(
                Flag64 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                const B1; #[component(name = "b2")] const B2; #[component(name = "b3")]
                const B3; #[component(name = "b4")] const B4; #[component(name = "b5")]
                const B5; #[component(name = "b6")] const B6; #[component(name = "b7")]
                const B7; #[component(name = "b8")] const B8; #[component(name = "b9")]
                const B9; #[component(name = "b10")] const B10; #[component(name =
                "b11")] const B11; #[component(name = "b12")] const B12; #[component(name
                = "b13")] const B13; #[component(name = "b14")] const B14;
                #[component(name = "b15")] const B15; #[component(name = "b16")] const
                B16; #[component(name = "b17")] const B17; #[component(name = "b18")]
                const B18; #[component(name = "b19")] const B19; #[component(name =
                "b20")] const B20; #[component(name = "b21")] const B21; #[component(name
                = "b22")] const B22; #[component(name = "b23")] const B23;
                #[component(name = "b24")] const B24; #[component(name = "b25")] const
                B25; #[component(name = "b26")] const B26; #[component(name = "b27")]
                const B27; #[component(name = "b28")] const B28; #[component(name =
                "b29")] const B29; #[component(name = "b30")] const B30; #[component(name
                = "b31")] const B31; #[component(name = "b32")] const B32;
                #[component(name = "b33")] const B33; #[component(name = "b34")] const
                B34; #[component(name = "b35")] const B35; #[component(name = "b36")]
                const B36; #[component(name = "b37")] const B37; #[component(name =
                "b38")] const B38; #[component(name = "b39")] const B39; #[component(name
                = "b40")] const B40; #[component(name = "b41")] const B41;
                #[component(name = "b42")] const B42; #[component(name = "b43")] const
                B43; #[component(name = "b44")] const B44; #[component(name = "b45")]
                const B45; #[component(name = "b46")] const B46; #[component(name =
                "b47")] const B47; #[component(name = "b48")] const B48; #[component(name
                = "b49")] const B49; #[component(name = "b50")] const B50;
                #[component(name = "b51")] const B51; #[component(name = "b52")] const
                B52; #[component(name = "b53")] const B53; #[component(name = "b54")]
                const B54; #[component(name = "b55")] const B55; #[component(name =
                "b56")] const B56; #[component(name = "b57")] const B57; #[component(name
                = "b58")] const B58; #[component(name = "b59")] const B59;
                #[component(name = "b60")] const B60; #[component(name = "b61")] const
                B61; #[component(name = "b62")] const B62; #[component(name = "b63")]
                const B63; }
            );
            const _: () = {
                assert!(8 == < Flag64 as wasmtime::component::ComponentType >::SIZE32);
                assert!(4 == < Flag64 as wasmtime::component::ComponentType >::ALIGN32);
            };
            pub trait Host {
                fn roundtrip_flag1(&mut self, x: Flag1) -> Flag1;
                fn roundtrip_flag2(&mut self, x: Flag2) -> Flag2;
                fn roundtrip_flag4(&mut self, x: Flag4) -> Flag4;
                fn roundtrip_flag8(&mut self, x: Flag8) -> Flag8;
                fn roundtrip_flag16(&mut self, x: Flag16) -> Flag16;
                fn roundtrip_flag32(&mut self, x: Flag32) -> Flag32;
                fn roundtrip_flag64(&mut self, x: Flag64) -> Flag64;
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
                let mut inst = linker.instance("foo:foo/flegs")?;
                inst.func_wrap(
                    "roundtrip-flag1",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag1,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag1(host, arg0);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "roundtrip-flag2",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag2,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag2(host, arg0);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "roundtrip-flag4",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag4,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag4(host, arg0);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "roundtrip-flag8",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag8,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag8(host, arg0);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "roundtrip-flag16",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag16,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag16(host, arg0);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "roundtrip-flag32",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag32,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag32(host, arg0);
                        Ok((r,))
                    },
                )?;
                inst.func_wrap(
                    "roundtrip-flag64",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (Flag64,)|
                    {
                        let host = &mut host_getter(caller.data_mut());
                        let r = Host::roundtrip_flag64(host, arg0);
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
                fn roundtrip_flag1(&mut self, x: Flag1) -> Flag1 {
                    Host::roundtrip_flag1(*self, x)
                }
                fn roundtrip_flag2(&mut self, x: Flag2) -> Flag2 {
                    Host::roundtrip_flag2(*self, x)
                }
                fn roundtrip_flag4(&mut self, x: Flag4) -> Flag4 {
                    Host::roundtrip_flag4(*self, x)
                }
                fn roundtrip_flag8(&mut self, x: Flag8) -> Flag8 {
                    Host::roundtrip_flag8(*self, x)
                }
                fn roundtrip_flag16(&mut self, x: Flag16) -> Flag16 {
                    Host::roundtrip_flag16(*self, x)
                }
                fn roundtrip_flag32(&mut self, x: Flag32) -> Flag32 {
                    Host::roundtrip_flag32(*self, x)
                }
                fn roundtrip_flag64(&mut self, x: Flag64) -> Flag64 {
                    Host::roundtrip_flag64(*self, x)
                }
            }
        }
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod flegs {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                wasmtime::component::flags!(
                    Flag1 { #[component(name = "b0")] const B0; }
                );
                const _: () = {
                    assert!(
                        1 == < Flag1 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < Flag1 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                wasmtime::component::flags!(
                    Flag2 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                    const B1; }
                );
                const _: () = {
                    assert!(
                        1 == < Flag2 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < Flag2 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                wasmtime::component::flags!(
                    Flag4 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                    const B1; #[component(name = "b2")] const B2; #[component(name =
                    "b3")] const B3; }
                );
                const _: () = {
                    assert!(
                        1 == < Flag4 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < Flag4 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                wasmtime::component::flags!(
                    Flag8 { #[component(name = "b0")] const B0; #[component(name = "b1")]
                    const B1; #[component(name = "b2")] const B2; #[component(name =
                    "b3")] const B3; #[component(name = "b4")] const B4; #[component(name
                    = "b5")] const B5; #[component(name = "b6")] const B6;
                    #[component(name = "b7")] const B7; }
                );
                const _: () = {
                    assert!(
                        1 == < Flag8 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        1 == < Flag8 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                wasmtime::component::flags!(
                    Flag16 { #[component(name = "b0")] const B0; #[component(name =
                    "b1")] const B1; #[component(name = "b2")] const B2; #[component(name
                    = "b3")] const B3; #[component(name = "b4")] const B4;
                    #[component(name = "b5")] const B5; #[component(name = "b6")] const
                    B6; #[component(name = "b7")] const B7; #[component(name = "b8")]
                    const B8; #[component(name = "b9")] const B9; #[component(name =
                    "b10")] const B10; #[component(name = "b11")] const B11;
                    #[component(name = "b12")] const B12; #[component(name = "b13")]
                    const B13; #[component(name = "b14")] const B14; #[component(name =
                    "b15")] const B15; }
                );
                const _: () = {
                    assert!(
                        2 == < Flag16 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        2 == < Flag16 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                wasmtime::component::flags!(
                    Flag32 { #[component(name = "b0")] const B0; #[component(name =
                    "b1")] const B1; #[component(name = "b2")] const B2; #[component(name
                    = "b3")] const B3; #[component(name = "b4")] const B4;
                    #[component(name = "b5")] const B5; #[component(name = "b6")] const
                    B6; #[component(name = "b7")] const B7; #[component(name = "b8")]
                    const B8; #[component(name = "b9")] const B9; #[component(name =
                    "b10")] const B10; #[component(name = "b11")] const B11;
                    #[component(name = "b12")] const B12; #[component(name = "b13")]
                    const B13; #[component(name = "b14")] const B14; #[component(name =
                    "b15")] const B15; #[component(name = "b16")] const B16;
                    #[component(name = "b17")] const B17; #[component(name = "b18")]
                    const B18; #[component(name = "b19")] const B19; #[component(name =
                    "b20")] const B20; #[component(name = "b21")] const B21;
                    #[component(name = "b22")] const B22; #[component(name = "b23")]
                    const B23; #[component(name = "b24")] const B24; #[component(name =
                    "b25")] const B25; #[component(name = "b26")] const B26;
                    #[component(name = "b27")] const B27; #[component(name = "b28")]
                    const B28; #[component(name = "b29")] const B29; #[component(name =
                    "b30")] const B30; #[component(name = "b31")] const B31; }
                );
                const _: () = {
                    assert!(
                        4 == < Flag32 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < Flag32 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                wasmtime::component::flags!(
                    Flag64 { #[component(name = "b0")] const B0; #[component(name =
                    "b1")] const B1; #[component(name = "b2")] const B2; #[component(name
                    = "b3")] const B3; #[component(name = "b4")] const B4;
                    #[component(name = "b5")] const B5; #[component(name = "b6")] const
                    B6; #[component(name = "b7")] const B7; #[component(name = "b8")]
                    const B8; #[component(name = "b9")] const B9; #[component(name =
                    "b10")] const B10; #[component(name = "b11")] const B11;
                    #[component(name = "b12")] const B12; #[component(name = "b13")]
                    const B13; #[component(name = "b14")] const B14; #[component(name =
                    "b15")] const B15; #[component(name = "b16")] const B16;
                    #[component(name = "b17")] const B17; #[component(name = "b18")]
                    const B18; #[component(name = "b19")] const B19; #[component(name =
                    "b20")] const B20; #[component(name = "b21")] const B21;
                    #[component(name = "b22")] const B22; #[component(name = "b23")]
                    const B23; #[component(name = "b24")] const B24; #[component(name =
                    "b25")] const B25; #[component(name = "b26")] const B26;
                    #[component(name = "b27")] const B27; #[component(name = "b28")]
                    const B28; #[component(name = "b29")] const B29; #[component(name =
                    "b30")] const B30; #[component(name = "b31")] const B31;
                    #[component(name = "b32")] const B32; #[component(name = "b33")]
                    const B33; #[component(name = "b34")] const B34; #[component(name =
                    "b35")] const B35; #[component(name = "b36")] const B36;
                    #[component(name = "b37")] const B37; #[component(name = "b38")]
                    const B38; #[component(name = "b39")] const B39; #[component(name =
                    "b40")] const B40; #[component(name = "b41")] const B41;
                    #[component(name = "b42")] const B42; #[component(name = "b43")]
                    const B43; #[component(name = "b44")] const B44; #[component(name =
                    "b45")] const B45; #[component(name = "b46")] const B46;
                    #[component(name = "b47")] const B47; #[component(name = "b48")]
                    const B48; #[component(name = "b49")] const B49; #[component(name =
                    "b50")] const B50; #[component(name = "b51")] const B51;
                    #[component(name = "b52")] const B52; #[component(name = "b53")]
                    const B53; #[component(name = "b54")] const B54; #[component(name =
                    "b55")] const B55; #[component(name = "b56")] const B56;
                    #[component(name = "b57")] const B57; #[component(name = "b58")]
                    const B58; #[component(name = "b59")] const B59; #[component(name =
                    "b60")] const B60; #[component(name = "b61")] const B61;
                    #[component(name = "b62")] const B62; #[component(name = "b63")]
                    const B63; }
                );
                const _: () = {
                    assert!(
                        8 == < Flag64 as wasmtime::component::ComponentType >::SIZE32
                    );
                    assert!(
                        4 == < Flag64 as wasmtime::component::ComponentType >::ALIGN32
                    );
                };
                pub struct Guest {
                    roundtrip_flag1: wasmtime::component::Func,
                    roundtrip_flag2: wasmtime::component::Func,
                    roundtrip_flag4: wasmtime::component::Func,
                    roundtrip_flag8: wasmtime::component::Func,
                    roundtrip_flag16: wasmtime::component::Func,
                    roundtrip_flag32: wasmtime::component::Func,
                    roundtrip_flag64: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let roundtrip_flag1 = *__exports
                            .typed_func::<(Flag1,), (Flag1,)>("roundtrip-flag1")?
                            .func();
                        let roundtrip_flag2 = *__exports
                            .typed_func::<(Flag2,), (Flag2,)>("roundtrip-flag2")?
                            .func();
                        let roundtrip_flag4 = *__exports
                            .typed_func::<(Flag4,), (Flag4,)>("roundtrip-flag4")?
                            .func();
                        let roundtrip_flag8 = *__exports
                            .typed_func::<(Flag8,), (Flag8,)>("roundtrip-flag8")?
                            .func();
                        let roundtrip_flag16 = *__exports
                            .typed_func::<(Flag16,), (Flag16,)>("roundtrip-flag16")?
                            .func();
                        let roundtrip_flag32 = *__exports
                            .typed_func::<(Flag32,), (Flag32,)>("roundtrip-flag32")?
                            .func();
                        let roundtrip_flag64 = *__exports
                            .typed_func::<(Flag64,), (Flag64,)>("roundtrip-flag64")?
                            .func();
                        Ok(Guest {
                            roundtrip_flag1,
                            roundtrip_flag2,
                            roundtrip_flag4,
                            roundtrip_flag8,
                            roundtrip_flag16,
                            roundtrip_flag32,
                            roundtrip_flag64,
                        })
                    }
                    pub fn call_roundtrip_flag1<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag1,
                    ) -> wasmtime::Result<Flag1> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag1,),
                                (Flag1,),
                            >::new_unchecked(self.roundtrip_flag1)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_roundtrip_flag2<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag2,
                    ) -> wasmtime::Result<Flag2> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag2,),
                                (Flag2,),
                            >::new_unchecked(self.roundtrip_flag2)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_roundtrip_flag4<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag4,
                    ) -> wasmtime::Result<Flag4> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag4,),
                                (Flag4,),
                            >::new_unchecked(self.roundtrip_flag4)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_roundtrip_flag8<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag8,
                    ) -> wasmtime::Result<Flag8> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag8,),
                                (Flag8,),
                            >::new_unchecked(self.roundtrip_flag8)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_roundtrip_flag16<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag16,
                    ) -> wasmtime::Result<Flag16> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag16,),
                                (Flag16,),
                            >::new_unchecked(self.roundtrip_flag16)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_roundtrip_flag32<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag32,
                    ) -> wasmtime::Result<Flag32> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag32,),
                                (Flag32,),
                            >::new_unchecked(self.roundtrip_flag32)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                    pub fn call_roundtrip_flag64<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: Flag64,
                    ) -> wasmtime::Result<Flag64> {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (Flag64,),
                                (Flag64,),
                            >::new_unchecked(self.roundtrip_flag64)
                        };
                        let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
                        callee.post_return(store.as_context_mut())?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
