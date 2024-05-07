pub struct TheLists {
    interface0: exports::foo::foo::lists::Guest,
}
const _: () = {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;
    impl TheLists {
        pub fn add_to_linker<T, U>(
            linker: &mut wasmtime::component::Linker<T>,
            get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> wasmtime::Result<()>
        where
            U: foo::foo::lists::Host + Send,
            T: Send,
        {
            foo::foo::lists::add_to_linker(linker, get)?;
            Ok(())
        }
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub async fn instantiate_async<T: Send>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate_async(&mut store, component).await?;
            Ok((Self::new(store, &instance)?, instance))
        }
        /// Instantiates a pre-instantiated module using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub async fn instantiate_pre<T: Send>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance_pre: &wasmtime::component::InstancePre<T>,
        ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {
            let instance = instance_pre.instantiate_async(&mut store).await?;
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
            let interface0 = exports::foo::foo::lists::Guest::new(
                &mut __exports
                    .instance("foo:foo/lists")
                    .ok_or_else(|| {
                        anyhow::anyhow!("exported instance `foo:foo/lists` not present")
                    })?,
            )?;
            Ok(TheLists { interface0 })
        }
        pub fn foo_foo_lists(&self) -> &exports::foo::foo::lists::Guest {
            &self.interface0
        }
    }
};
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod lists {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::anyhow;
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct OtherRecord {
                #[component(name = "a1")]
                pub a1: u32,
                #[component(name = "a2")]
                pub a2: u64,
                #[component(name = "a3")]
                pub a3: i32,
                #[component(name = "a4")]
                pub a4: i64,
                #[component(name = "b")]
                pub b: wasmtime::component::__internal::String,
                #[component(name = "c")]
                pub c: wasmtime::component::__internal::Vec<u8>,
            }
            impl core::fmt::Debug for OtherRecord {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("OtherRecord")
                        .field("a1", &self.a1)
                        .field("a2", &self.a2)
                        .field("a3", &self.a3)
                        .field("a4", &self.a4)
                        .field("b", &self.b)
                        .field("c", &self.c)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    48 == < OtherRecord as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    8 == < OtherRecord as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(record)]
            #[derive(Clone)]
            pub struct SomeRecord {
                #[component(name = "x")]
                pub x: wasmtime::component::__internal::String,
                #[component(name = "y")]
                pub y: OtherRecord,
                #[component(name = "z")]
                pub z: wasmtime::component::__internal::Vec<OtherRecord>,
                #[component(name = "c1")]
                pub c1: u32,
                #[component(name = "c2")]
                pub c2: u64,
                #[component(name = "c3")]
                pub c3: i32,
                #[component(name = "c4")]
                pub c4: i64,
            }
            impl core::fmt::Debug for SomeRecord {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct("SomeRecord")
                        .field("x", &self.x)
                        .field("y", &self.y)
                        .field("z", &self.z)
                        .field("c1", &self.c1)
                        .field("c2", &self.c2)
                        .field("c3", &self.c3)
                        .field("c4", &self.c4)
                        .finish()
                }
            }
            const _: () = {
                assert!(
                    96 == < SomeRecord as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    8 == < SomeRecord as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum OtherVariant {
                #[component(name = "a")]
                A,
                #[component(name = "b")]
                B(u32),
                #[component(name = "c")]
                C(wasmtime::component::__internal::String),
            }
            impl core::fmt::Debug for OtherVariant {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        OtherVariant::A => f.debug_tuple("OtherVariant::A").finish(),
                        OtherVariant::B(e) => {
                            f.debug_tuple("OtherVariant::B").field(e).finish()
                        }
                        OtherVariant::C(e) => {
                            f.debug_tuple("OtherVariant::C").field(e).finish()
                        }
                    }
                }
            }
            const _: () = {
                assert!(
                    12 == < OtherVariant as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < OtherVariant as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            #[derive(wasmtime::component::ComponentType)]
            #[derive(wasmtime::component::Lift)]
            #[derive(wasmtime::component::Lower)]
            #[component(variant)]
            #[derive(Clone)]
            pub enum SomeVariant {
                #[component(name = "a")]
                A(wasmtime::component::__internal::String),
                #[component(name = "b")]
                B,
                #[component(name = "c")]
                C(u32),
                #[component(name = "d")]
                D(wasmtime::component::__internal::Vec<OtherVariant>),
            }
            impl core::fmt::Debug for SomeVariant {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        SomeVariant::A(e) => {
                            f.debug_tuple("SomeVariant::A").field(e).finish()
                        }
                        SomeVariant::B => f.debug_tuple("SomeVariant::B").finish(),
                        SomeVariant::C(e) => {
                            f.debug_tuple("SomeVariant::C").field(e).finish()
                        }
                        SomeVariant::D(e) => {
                            f.debug_tuple("SomeVariant::D").field(e).finish()
                        }
                    }
                }
            }
            const _: () = {
                assert!(
                    12 == < SomeVariant as wasmtime::component::ComponentType >::SIZE32
                );
                assert!(
                    4 == < SomeVariant as wasmtime::component::ComponentType >::ALIGN32
                );
            };
            pub type LoadStoreAllSizes = wasmtime::component::__internal::Vec<
                (
                    wasmtime::component::__internal::String,
                    u8,
                    i8,
                    u16,
                    i16,
                    u32,
                    i32,
                    u64,
                    i64,
                    f32,
                    f64,
                    char,
                ),
            >;
            const _: () = {
                assert!(
                    8 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                    >::SIZE32
                );
                assert!(
                    4 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                    >::ALIGN32
                );
            };
            #[wasmtime::component::__internal::async_trait]
            pub trait Host {
                async fn list_u8_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u8>,
                ) -> ();
                async fn list_u16_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u16>,
                ) -> ();
                async fn list_u32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u32>,
                ) -> ();
                async fn list_u64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<u64>,
                ) -> ();
                async fn list_s8_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i8>,
                ) -> ();
                async fn list_s16_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i16>,
                ) -> ();
                async fn list_s32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i32>,
                ) -> ();
                async fn list_s64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<i64>,
                ) -> ();
                async fn list_float32_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<f32>,
                ) -> ();
                async fn list_float64_param(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<f64>,
                ) -> ();
                async fn list_u8_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u8>;
                async fn list_u16_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u16>;
                async fn list_u32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u32>;
                async fn list_u64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<u64>;
                async fn list_s8_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i8>;
                async fn list_s16_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i16>;
                async fn list_s32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i32>;
                async fn list_s64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<i64>;
                async fn list_float32_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<f32>;
                async fn list_float64_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<f64>;
                async fn tuple_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<(u8, i8)>,
                ) -> wasmtime::component::__internal::Vec<(i64, u32)>;
                async fn string_list_arg(
                    &mut self,
                    a: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::String,
                    >,
                ) -> ();
                async fn string_list_ret(
                    &mut self,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::String,
                >;
                async fn tuple_string_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        (u8, wasmtime::component::__internal::String),
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    (wasmtime::component::__internal::String, u8),
                >;
                async fn string_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<
                        wasmtime::component::__internal::String,
                    >,
                ) -> wasmtime::component::__internal::Vec<
                    wasmtime::component::__internal::String,
                >;
                async fn record_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<SomeRecord>,
                ) -> wasmtime::component::__internal::Vec<OtherRecord>;
                async fn record_list_reverse(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<OtherRecord>,
                ) -> wasmtime::component::__internal::Vec<SomeRecord>;
                async fn variant_list(
                    &mut self,
                    x: wasmtime::component::__internal::Vec<SomeVariant>,
                ) -> wasmtime::component::__internal::Vec<OtherVariant>;
                async fn load_store_everything(
                    &mut self,
                    a: LoadStoreAllSizes,
                ) -> LoadStoreAllSizes;
            }
            pub fn add_to_linker<T, U>(
                linker: &mut wasmtime::component::Linker<T>,
                get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
            ) -> wasmtime::Result<()>
            where
                T: Send,
                U: Host + Send,
            {
                let mut inst = linker.instance("foo:foo/lists")?;
                inst.func_wrap_async(
                    "list-u8-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u8>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u8_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u16-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u16>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u16_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u32-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u32>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u32_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u64-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<u64>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u64_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s8-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i8>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s8_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s16-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i16>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s16_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s32-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i32>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s32_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s64-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<i64>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s64_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-float32-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<f32>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_float32_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-float64-param",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<f64>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_float64_param(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u8-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u8_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u16-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u16_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u32-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u32_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-u64-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_u64_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s8-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s8_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s16-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s16_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s32-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s32_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-s64-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_s64_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-float32-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_float32_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "list-float64-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::list_float64_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "tuple-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<(u8, i8)>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::tuple_list(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "string-list-arg",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::String,
                            >,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::string_list_arg(host, arg0).await;
                        Ok(r)
                    }),
                )?;
                inst.func_wrap_async(
                    "string-list-ret",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::string_list_ret(host).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "tuple-string-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                (u8, wasmtime::component::__internal::String),
                            >,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::tuple_string_list(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "string-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (
                            arg0,
                        ): (
                            wasmtime::component::__internal::Vec<
                                wasmtime::component::__internal::String,
                            >,
                        )|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::string_list(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "record-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<SomeRecord>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::record_list(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "record-list-reverse",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<OtherRecord>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::record_list_reverse(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "variant-list",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (wasmtime::component::__internal::Vec<SomeVariant>,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::variant_list(host, arg0).await;
                        Ok((r,))
                    }),
                )?;
                inst.func_wrap_async(
                    "load-store-everything",
                    move |
                        mut caller: wasmtime::StoreContextMut<'_, T>,
                        (arg0,): (LoadStoreAllSizes,)|
                    wasmtime::component::__internal::Box::new(async move {
                        let host = get(caller.data_mut());
                        let r = Host::load_store_everything(host, arg0).await;
                        Ok((r,))
                    }),
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
            pub mod lists {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::anyhow;
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct OtherRecord {
                    #[component(name = "a1")]
                    pub a1: u32,
                    #[component(name = "a2")]
                    pub a2: u64,
                    #[component(name = "a3")]
                    pub a3: i32,
                    #[component(name = "a4")]
                    pub a4: i64,
                    #[component(name = "b")]
                    pub b: wasmtime::component::__internal::String,
                    #[component(name = "c")]
                    pub c: wasmtime::component::__internal::Vec<u8>,
                }
                impl core::fmt::Debug for OtherRecord {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("OtherRecord")
                            .field("a1", &self.a1)
                            .field("a2", &self.a2)
                            .field("a3", &self.a3)
                            .field("a4", &self.a4)
                            .field("b", &self.b)
                            .field("c", &self.c)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        48 == < OtherRecord as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        8 == < OtherRecord as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(record)]
                #[derive(Clone)]
                pub struct SomeRecord {
                    #[component(name = "x")]
                    pub x: wasmtime::component::__internal::String,
                    #[component(name = "y")]
                    pub y: OtherRecord,
                    #[component(name = "z")]
                    pub z: wasmtime::component::__internal::Vec<OtherRecord>,
                    #[component(name = "c1")]
                    pub c1: u32,
                    #[component(name = "c2")]
                    pub c2: u64,
                    #[component(name = "c3")]
                    pub c3: i32,
                    #[component(name = "c4")]
                    pub c4: i64,
                }
                impl core::fmt::Debug for SomeRecord {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        f.debug_struct("SomeRecord")
                            .field("x", &self.x)
                            .field("y", &self.y)
                            .field("z", &self.z)
                            .field("c1", &self.c1)
                            .field("c2", &self.c2)
                            .field("c3", &self.c3)
                            .field("c4", &self.c4)
                            .finish()
                    }
                }
                const _: () = {
                    assert!(
                        96 == < SomeRecord as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        8 == < SomeRecord as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone)]
                pub enum OtherVariant {
                    #[component(name = "a")]
                    A,
                    #[component(name = "b")]
                    B(u32),
                    #[component(name = "c")]
                    C(wasmtime::component::__internal::String),
                }
                impl core::fmt::Debug for OtherVariant {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            OtherVariant::A => f.debug_tuple("OtherVariant::A").finish(),
                            OtherVariant::B(e) => {
                                f.debug_tuple("OtherVariant::B").field(e).finish()
                            }
                            OtherVariant::C(e) => {
                                f.debug_tuple("OtherVariant::C").field(e).finish()
                            }
                        }
                    }
                }
                const _: () = {
                    assert!(
                        12 == < OtherVariant as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < OtherVariant as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                #[derive(wasmtime::component::ComponentType)]
                #[derive(wasmtime::component::Lift)]
                #[derive(wasmtime::component::Lower)]
                #[component(variant)]
                #[derive(Clone)]
                pub enum SomeVariant {
                    #[component(name = "a")]
                    A(wasmtime::component::__internal::String),
                    #[component(name = "b")]
                    B,
                    #[component(name = "c")]
                    C(u32),
                    #[component(name = "d")]
                    D(wasmtime::component::__internal::Vec<OtherVariant>),
                }
                impl core::fmt::Debug for SomeVariant {
                    fn fmt(
                        &self,
                        f: &mut core::fmt::Formatter<'_>,
                    ) -> core::fmt::Result {
                        match self {
                            SomeVariant::A(e) => {
                                f.debug_tuple("SomeVariant::A").field(e).finish()
                            }
                            SomeVariant::B => f.debug_tuple("SomeVariant::B").finish(),
                            SomeVariant::C(e) => {
                                f.debug_tuple("SomeVariant::C").field(e).finish()
                            }
                            SomeVariant::D(e) => {
                                f.debug_tuple("SomeVariant::D").field(e).finish()
                            }
                        }
                    }
                }
                const _: () = {
                    assert!(
                        12 == < SomeVariant as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < SomeVariant as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub type LoadStoreAllSizes = wasmtime::component::__internal::Vec<
                    (
                        wasmtime::component::__internal::String,
                        u8,
                        i8,
                        u16,
                        i16,
                        u32,
                        i32,
                        u64,
                        i64,
                        f32,
                        f64,
                        char,
                    ),
                >;
                const _: () = {
                    assert!(
                        8 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                        >::SIZE32
                    );
                    assert!(
                        4 == < LoadStoreAllSizes as wasmtime::component::ComponentType
                        >::ALIGN32
                    );
                };
                pub struct Guest {
                    list_u8_param: wasmtime::component::Func,
                    list_u16_param: wasmtime::component::Func,
                    list_u32_param: wasmtime::component::Func,
                    list_u64_param: wasmtime::component::Func,
                    list_s8_param: wasmtime::component::Func,
                    list_s16_param: wasmtime::component::Func,
                    list_s32_param: wasmtime::component::Func,
                    list_s64_param: wasmtime::component::Func,
                    list_float32_param: wasmtime::component::Func,
                    list_float64_param: wasmtime::component::Func,
                    list_u8_ret: wasmtime::component::Func,
                    list_u16_ret: wasmtime::component::Func,
                    list_u32_ret: wasmtime::component::Func,
                    list_u64_ret: wasmtime::component::Func,
                    list_s8_ret: wasmtime::component::Func,
                    list_s16_ret: wasmtime::component::Func,
                    list_s32_ret: wasmtime::component::Func,
                    list_s64_ret: wasmtime::component::Func,
                    list_float32_ret: wasmtime::component::Func,
                    list_float64_ret: wasmtime::component::Func,
                    tuple_list: wasmtime::component::Func,
                    string_list_arg: wasmtime::component::Func,
                    string_list_ret: wasmtime::component::Func,
                    tuple_string_list: wasmtime::component::Func,
                    string_list: wasmtime::component::Func,
                    record_list: wasmtime::component::Func,
                    record_list_reverse: wasmtime::component::Func,
                    variant_list: wasmtime::component::Func,
                    load_store_everything: wasmtime::component::Func,
                }
                impl Guest {
                    pub fn new(
                        __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                    ) -> wasmtime::Result<Guest> {
                        let list_u8_param = *__exports
                            .typed_func::<(&[u8],), ()>("list-u8-param")?
                            .func();
                        let list_u16_param = *__exports
                            .typed_func::<(&[u16],), ()>("list-u16-param")?
                            .func();
                        let list_u32_param = *__exports
                            .typed_func::<(&[u32],), ()>("list-u32-param")?
                            .func();
                        let list_u64_param = *__exports
                            .typed_func::<(&[u64],), ()>("list-u64-param")?
                            .func();
                        let list_s8_param = *__exports
                            .typed_func::<(&[i8],), ()>("list-s8-param")?
                            .func();
                        let list_s16_param = *__exports
                            .typed_func::<(&[i16],), ()>("list-s16-param")?
                            .func();
                        let list_s32_param = *__exports
                            .typed_func::<(&[i32],), ()>("list-s32-param")?
                            .func();
                        let list_s64_param = *__exports
                            .typed_func::<(&[i64],), ()>("list-s64-param")?
                            .func();
                        let list_float32_param = *__exports
                            .typed_func::<(&[f32],), ()>("list-float32-param")?
                            .func();
                        let list_float64_param = *__exports
                            .typed_func::<(&[f64],), ()>("list-float64-param")?
                            .func();
                        let list_u8_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u8>,),
                            >("list-u8-ret")?
                            .func();
                        let list_u16_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u16>,),
                            >("list-u16-ret")?
                            .func();
                        let list_u32_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >("list-u32-ret")?
                            .func();
                        let list_u64_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<u64>,),
                            >("list-u64-ret")?
                            .func();
                        let list_s8_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i8>,),
                            >("list-s8-ret")?
                            .func();
                        let list_s16_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i16>,),
                            >("list-s16-ret")?
                            .func();
                        let list_s32_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i32>,),
                            >("list-s32-ret")?
                            .func();
                        let list_s64_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<i64>,),
                            >("list-s64-ret")?
                            .func();
                        let list_float32_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<f32>,),
                            >("list-float32-ret")?
                            .func();
                        let list_float64_ret = *__exports
                            .typed_func::<
                                (),
                                (wasmtime::component::__internal::Vec<f64>,),
                            >("list-float64-ret")?
                            .func();
                        let tuple_list = *__exports
                            .typed_func::<
                                (&[(u8, i8)],),
                                (wasmtime::component::__internal::Vec<(i64, u32)>,),
                            >("tuple-list")?
                            .func();
                        let string_list_arg = *__exports
                            .typed_func::<
                                (&[wasmtime::component::__internal::String],),
                                (),
                            >("string-list-arg")?
                            .func();
                        let string_list_ret = *__exports
                            .typed_func::<
                                (),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >("string-list-ret")?
                            .func();
                        let tuple_string_list = *__exports
                            .typed_func::<
                                (&[(u8, wasmtime::component::__internal::String)],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        (wasmtime::component::__internal::String, u8),
                                    >,
                                ),
                            >("tuple-string-list")?
                            .func();
                        let string_list = *__exports
                            .typed_func::<
                                (&[wasmtime::component::__internal::String],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >("string-list")?
                            .func();
                        let record_list = *__exports
                            .typed_func::<
                                (&[SomeRecord],),
                                (wasmtime::component::__internal::Vec<OtherRecord>,),
                            >("record-list")?
                            .func();
                        let record_list_reverse = *__exports
                            .typed_func::<
                                (&[OtherRecord],),
                                (wasmtime::component::__internal::Vec<SomeRecord>,),
                            >("record-list-reverse")?
                            .func();
                        let variant_list = *__exports
                            .typed_func::<
                                (&[SomeVariant],),
                                (wasmtime::component::__internal::Vec<OtherVariant>,),
                            >("variant-list")?
                            .func();
                        let load_store_everything = *__exports
                            .typed_func::<
                                (&LoadStoreAllSizes,),
                                (LoadStoreAllSizes,),
                            >("load-store-everything")?
                            .func();
                        Ok(Guest {
                            list_u8_param,
                            list_u16_param,
                            list_u32_param,
                            list_u64_param,
                            list_s8_param,
                            list_s16_param,
                            list_s32_param,
                            list_s64_param,
                            list_float32_param,
                            list_float64_param,
                            list_u8_ret,
                            list_u16_ret,
                            list_u32_ret,
                            list_u64_ret,
                            list_s8_ret,
                            list_s16_ret,
                            list_s32_ret,
                            list_s64_ret,
                            list_float32_ret,
                            list_float64_ret,
                            tuple_list,
                            string_list_arg,
                            string_list_ret,
                            tuple_string_list,
                            string_list,
                            record_list,
                            record_list_reverse,
                            variant_list,
                            load_store_everything,
                        })
                    }
                    pub async fn call_list_u8_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u8],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u8],),
                                (),
                            >::new_unchecked(self.list_u8_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u16_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u16],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u16],),
                                (),
                            >::new_unchecked(self.list_u16_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u32],),
                                (),
                            >::new_unchecked(self.list_u32_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[u64],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[u64],),
                                (),
                            >::new_unchecked(self.list_u64_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s8_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i8],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i8],),
                                (),
                            >::new_unchecked(self.list_s8_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s16_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i16],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i16],),
                                (),
                            >::new_unchecked(self.list_s16_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i32],),
                                (),
                            >::new_unchecked(self.list_s32_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_s64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[i64],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[i64],),
                                (),
                            >::new_unchecked(self.list_s64_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_float32_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[f32],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[f32],),
                                (),
                            >::new_unchecked(self.list_float32_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_float64_param<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[f64],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[f64],),
                                (),
                            >::new_unchecked(self.list_float64_param)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_list_u8_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u8>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u8>,),
                            >::new_unchecked(self.list_u8_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_u16_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u16>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u16>,),
                            >::new_unchecked(self.list_u16_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_u32_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u32>,),
                            >::new_unchecked(self.list_u32_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_u64_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u64>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<u64>,),
                            >::new_unchecked(self.list_u64_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s8_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i8>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i8>,),
                            >::new_unchecked(self.list_s8_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s16_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i16>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i16>,),
                            >::new_unchecked(self.list_s16_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s32_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i32>,),
                            >::new_unchecked(self.list_s32_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_s64_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<i64>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<i64>,),
                            >::new_unchecked(self.list_s64_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_float32_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<f32>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<f32>,),
                            >::new_unchecked(self.list_float32_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_list_float64_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<f64>>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (wasmtime::component::__internal::Vec<f64>,),
                            >::new_unchecked(self.list_float64_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_tuple_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[(u8, i8)],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<(i64, u32)>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[(u8, i8)],),
                                (wasmtime::component::__internal::Vec<(i64, u32)>,),
                            >::new_unchecked(self.tuple_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_string_list_arg<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[wasmtime::component::__internal::String],
                    ) -> wasmtime::Result<()>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[wasmtime::component::__internal::String],),
                                (),
                            >::new_unchecked(self.string_list_arg)
                        };
                        let () = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(())
                    }
                    pub async fn call_string_list_ret<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::String,
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >::new_unchecked(self.string_list_ret)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), ())
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_tuple_string_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[(u8, wasmtime::component::__internal::String)],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            (wasmtime::component::__internal::String, u8),
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[(u8, wasmtime::component::__internal::String)],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        (wasmtime::component::__internal::String, u8),
                                    >,
                                ),
                            >::new_unchecked(self.tuple_string_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_string_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[wasmtime::component::__internal::String],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<
                            wasmtime::component::__internal::String,
                        >,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[wasmtime::component::__internal::String],),
                                (
                                    wasmtime::component::__internal::Vec<
                                        wasmtime::component::__internal::String,
                                    >,
                                ),
                            >::new_unchecked(self.string_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_record_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[SomeRecord],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<OtherRecord>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[SomeRecord],),
                                (wasmtime::component::__internal::Vec<OtherRecord>,),
                            >::new_unchecked(self.record_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_record_list_reverse<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[OtherRecord],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<SomeRecord>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[OtherRecord],),
                                (wasmtime::component::__internal::Vec<SomeRecord>,),
                            >::new_unchecked(self.record_list_reverse)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_variant_list<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &[SomeVariant],
                    ) -> wasmtime::Result<
                        wasmtime::component::__internal::Vec<OtherVariant>,
                    >
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&[SomeVariant],),
                                (wasmtime::component::__internal::Vec<OtherVariant>,),
                            >::new_unchecked(self.variant_list)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                    pub async fn call_load_store_everything<S: wasmtime::AsContextMut>(
                        &self,
                        mut store: S,
                        arg0: &LoadStoreAllSizes,
                    ) -> wasmtime::Result<LoadStoreAllSizes>
                    where
                        <S as wasmtime::AsContext>::Data: Send,
                    {
                        let callee = unsafe {
                            wasmtime::component::TypedFunc::<
                                (&LoadStoreAllSizes,),
                                (LoadStoreAllSizes,),
                            >::new_unchecked(self.load_store_everything)
                        };
                        let (ret0,) = callee
                            .call_async(store.as_context_mut(), (arg0,))
                            .await?;
                        callee.post_return_async(store.as_context_mut()).await?;
                        Ok(ret0)
                    }
                }
            }
        }
    }
}
