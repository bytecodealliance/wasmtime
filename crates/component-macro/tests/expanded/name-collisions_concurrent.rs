/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `name-collisions`.
///
/// This structure is created through [`NameCollisionsPre::new`] which
/// takes a [`InstancePre`](wasmtime::component::InstancePre) that
/// has been created through a [`Linker`](wasmtime::component::Linker).
///
/// For more information see [`NameCollisions`] as well.
pub struct NameCollisionsPre<T: 'static> {
    instance_pre: wasmtime::component::InstancePre<T>,
    indices: NameCollisionsIndices,
}
impl<T: 'static> Clone for NameCollisionsPre<T> {
    fn clone(&self) -> Self {
        Self {
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }
    }
}
impl<_T: 'static> NameCollisionsPre<_T> {
    /// Creates a new copy of `NameCollisionsPre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(
        instance_pre: wasmtime::component::InstancePre<_T>,
    ) -> wasmtime::Result<Self> {
        let indices = NameCollisionsIndices::new(&instance_pre)?;
        Ok(Self { instance_pre, indices })
    }
    pub fn engine(&self) -> &wasmtime::Engine {
        self.instance_pre.engine()
    }
    pub fn instance_pre(&self) -> &wasmtime::component::InstancePre<_T> {
        &self.instance_pre
    }
    /// Instantiates a new instance of [`NameCollisions`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub fn instantiate(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<NameCollisions> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate(&mut store)?;
        self.indices.load(&mut store, &instance)
    }
}
impl<_T: Send + 'static> NameCollisionsPre<_T> {
    /// Same as [`Self::instantiate`], except with `async`.
    pub async fn instantiate_async(
        &self,
        mut store: impl wasmtime::AsContextMut<Data = _T>,
    ) -> wasmtime::Result<NameCollisions> {
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        self.indices.load(&mut store, &instance)
    }
}
/// Auto-generated bindings for index of the exports of
/// `name-collisions`.
///
/// This is an implementation detail of [`NameCollisionsPre`] and can
/// be constructed if needed as well.
///
/// For more information see [`NameCollisions`] as well.
#[derive(Clone)]
pub struct NameCollisionsIndices {
    interface10: exports::instantiate_async::GuestIndices,
    interface11: exports::add_to_linker::GuestIndices,
    interface12: exports::add_to_linker_imports::GuestIndices,
    interface13: exports::call_run::GuestIndices,
    interface4: wasmtime::component::ComponentExportIndex,
    interface4_: exports::use_::fn_::self_::GuestIndices,
    interface5: exports::foo::foo::greeter::GuestIndices,
    interface6: exports::foo_::GuestIndices,
    interface7: exports::foo_foo_greeter::GuestIndices,
    interface8: exports::new::GuestIndices,
    interface9: exports::instantiate::GuestIndices,
    run: wasmtime::component::ComponentExportIndex,
    store_: wasmtime::component::ComponentExportIndex,
    type_: wasmtime::component::ComponentExportIndex,
}
/// Auto-generated bindings for an instance a component which
/// implements the world `name-collisions`.
///
/// This structure can be created through a number of means
/// depending on your requirements and what you have on hand:
///
/// * The most convenient way is to use
///   [`NameCollisions::instantiate`] which only needs a
///   [`Store`], [`Component`], and [`Linker`].
///
/// * Alternatively you can create a [`NameCollisionsPre`] ahead of
///   time with a [`Component`] to front-load string lookups
///   of exports once instead of per-instantiation. This
///   method then uses [`NameCollisionsPre::instantiate`] to
///   create a [`NameCollisions`].
///
/// * If you've instantiated the instance yourself already
///   then you can use [`NameCollisions::new`].
///
/// These methods are all equivalent to one another and move
/// around the tradeoff of what work is performed when.
///
/// [`Store`]: wasmtime::Store
/// [`Component`]: wasmtime::component::Component
/// [`Linker`]: wasmtime::component::Linker
pub struct NameCollisions {
    interface10: exports::instantiate_async::Guest,
    interface11: exports::add_to_linker::Guest,
    interface12: exports::add_to_linker_imports::Guest,
    interface13: exports::call_run::Guest,
    interface4: wasmtime::component::Func,
    interface4_: exports::use_::fn_::self_::Guest,
    interface5: exports::foo::foo::greeter::Guest,
    interface6: exports::foo_::Guest,
    interface7: exports::foo_foo_greeter::Guest,
    interface8: exports::new::Guest,
    interface9: exports::instantiate::Guest,
    run: wasmtime::component::Func,
    store_: wasmtime::component::Func,
    type_: wasmtime::component::Func,
}
const _: () = {
    impl NameCollisionsIndices {
        /// Creates a new copy of `NameCollisionsIndices` bindings which can then
        /// be used to instantiate into a particular store.
        ///
        /// This method may fail if the component does not have the
        /// required exports.
        pub fn new<_T>(
            _instance_pre: &wasmtime::component::InstancePre<_T>,
        ) -> wasmtime::Result<Self> {
            let _component = _instance_pre.component();
            let _instance_type = _instance_pre.instance_type();
            let interface10 = exports::instantiate_async::GuestIndices::new(
                _instance_pre,
            )?;
            let interface11 = exports::add_to_linker::GuestIndices::new(_instance_pre)?;
            let interface12 = exports::add_to_linker_imports::GuestIndices::new(
                _instance_pre,
            )?;
            let interface13 = exports::call_run::GuestIndices::new(_instance_pre)?;
            let interface4 = {
                let (item, index) = _component
                    .get_export(None, "interface4")
                    .ok_or_else(|| {
                        wasmtime::format_err!("no export `interface4` found")
                    })?;
                match item {
                    wasmtime::component::types::ComponentItem::ComponentFunc(func) => {
                        wasmtime::error::Context::context(
                            func.typecheck::<(), ()>(&_instance_type),
                            "type-checking export func `interface4`",
                        )?;
                        index
                    }
                    _ => {
                        Err(
                            wasmtime::format_err!(
                                "export `interface4` is not a function"
                            ),
                        )?
                    }
                }
            };
            let interface4_ = exports::use_::fn_::self_::GuestIndices::new(
                _instance_pre,
            )?;
            let interface5 = exports::foo::foo::greeter::GuestIndices::new(
                _instance_pre,
            )?;
            let interface6 = exports::foo_::GuestIndices::new(_instance_pre)?;
            let interface7 = exports::foo_foo_greeter::GuestIndices::new(_instance_pre)?;
            let interface8 = exports::new::GuestIndices::new(_instance_pre)?;
            let interface9 = exports::instantiate::GuestIndices::new(_instance_pre)?;
            let run = {
                let (item, index) = _component
                    .get_export(None, "run")
                    .ok_or_else(|| wasmtime::format_err!("no export `run` found"))?;
                match item {
                    wasmtime::component::types::ComponentItem::ComponentFunc(func) => {
                        wasmtime::error::Context::context(
                            func.typecheck::<(), ()>(&_instance_type),
                            "type-checking export func `run`",
                        )?;
                        index
                    }
                    _ => Err(wasmtime::format_err!("export `run` is not a function"))?,
                }
            };
            let store_ = {
                let (item, index) = _component
                    .get_export(None, "store")
                    .ok_or_else(|| wasmtime::format_err!("no export `store` found"))?;
                match item {
                    wasmtime::component::types::ComponentItem::ComponentFunc(func) => {
                        wasmtime::error::Context::context(
                            func.typecheck::<(), ()>(&_instance_type),
                            "type-checking export func `store`",
                        )?;
                        index
                    }
                    _ => Err(wasmtime::format_err!("export `store` is not a function"))?,
                }
            };
            let type_ = {
                let (item, index) = _component
                    .get_export(None, "type")
                    .ok_or_else(|| wasmtime::format_err!("no export `type` found"))?;
                match item {
                    wasmtime::component::types::ComponentItem::ComponentFunc(func) => {
                        wasmtime::error::Context::context(
                            func.typecheck::<(), ()>(&_instance_type),
                            "type-checking export func `type`",
                        )?;
                        index
                    }
                    _ => Err(wasmtime::format_err!("export `type` is not a function"))?,
                }
            };
            Ok(NameCollisionsIndices {
                interface10,
                interface11,
                interface12,
                interface13,
                interface4,
                interface4_,
                interface5,
                interface6,
                interface7,
                interface8,
                interface9,
                run,
                store_,
                type_,
            })
        }
        /// Uses the indices stored in `self` to load an instance
        /// of [`NameCollisions`] from the instance provided.
        ///
        /// Note that at this time this method will additionally
        /// perform type-checks of all exports.
        pub fn load(
            &self,
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<NameCollisions> {
            let _ = &mut store;
            let _instance = instance;
            let interface10 = self.interface10.load(&mut store, &_instance)?;
            let interface11 = self.interface11.load(&mut store, &_instance)?;
            let interface12 = self.interface12.load(&mut store, &_instance)?;
            let interface13 = self.interface13.load(&mut store, &_instance)?;
            let interface4 = *_instance
                .get_typed_func::<(), ()>(&mut store, &self.interface4)?
                .func();
            let interface4_ = self.interface4_.load(&mut store, &_instance)?;
            let interface5 = self.interface5.load(&mut store, &_instance)?;
            let interface6 = self.interface6.load(&mut store, &_instance)?;
            let interface7 = self.interface7.load(&mut store, &_instance)?;
            let interface8 = self.interface8.load(&mut store, &_instance)?;
            let interface9 = self.interface9.load(&mut store, &_instance)?;
            let run = *_instance.get_typed_func::<(), ()>(&mut store, &self.run)?.func();
            let store_ = *_instance
                .get_typed_func::<(), ()>(&mut store, &self.store_)?
                .func();
            let type_ = *_instance
                .get_typed_func::<(), ()>(&mut store, &self.type_)?
                .func();
            Ok(NameCollisions {
                interface10,
                interface11,
                interface12,
                interface13,
                interface4,
                interface4_,
                interface5,
                interface6,
                interface7,
                interface8,
                interface9,
                run,
                store_,
                type_,
            })
        }
    }
    impl NameCollisions {
        /// Convenience wrapper around [`NameCollisionsPre::new`] and
        /// [`NameCollisionsPre::instantiate`].
        pub fn instantiate<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<NameCollisions> {
            let pre = linker.instantiate_pre(component)?;
            NameCollisionsPre::new(pre)?.instantiate(store)
        }
        /// Convenience wrapper around [`NameCollisionsIndices::new`] and
        /// [`NameCollisionsIndices::load`].
        pub fn new(
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> wasmtime::Result<NameCollisions> {
            let indices = NameCollisionsIndices::new(&instance.instance_pre(&store))?;
            indices.load(&mut store, instance)
        }
        /// Convenience wrapper around [`NameCollisionsPre::new`] and
        /// [`NameCollisionsPre::instantiate_async`].
        pub async fn instantiate_async<_T>(
            store: impl wasmtime::AsContextMut<Data = _T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<_T>,
        ) -> wasmtime::Result<NameCollisions>
        where
            _T: Send,
        {
            let pre = linker.instantiate_pre(component)?;
            NameCollisionsPre::new(pre)?.instantiate_async(store).await
        }
        pub fn add_to_linker<T, D>(
            linker: &mut wasmtime::component::Linker<T>,
            host_getter: fn(&mut T) -> D::Data<'_>,
        ) -> wasmtime::Result<()>
        where
            D: foo::foo::store::HostWithStore<T> + exports_::bar::baz::HostWithStore<T>
                + foo_::HostWithStore<T> + exports__::HostWithStore<T>
                + named_imports_::HostWithStore<T> + use_::fn_::self_::HostWithStore<T>
                + type_::HostWithStore<T> + self_::HostWithStore<T>
                + foo::foo::store::HostWithStore<T> + Send,
            for<'a> D::Data<
                'a,
            >: foo::foo::store::Host + exports_::bar::baz::Host + foo_::Host
                + exports__::Host + named_imports_::Host + use_::fn_::self_::Host
                + type_::Host + self_::Host + foo::foo::store::Host + Send,
            T: 'static + Send,
        {
            foo::foo::store::add_to_linker::<T, D>(linker, host_getter)?;
            exports_::bar::baz::add_to_linker::<T, D>(linker, host_getter)?;
            foo_::add_to_linker::<T, D>(linker, host_getter)?;
            exports__::add_to_linker::<T, D>(linker, host_getter)?;
            named_imports_::add_to_linker::<T, D>(linker, host_getter)?;
            use_::fn_::self_::add_to_linker::<T, D>(linker, host_getter)?;
            type_::add_to_linker::<T, D>(linker, host_getter)?;
            self_::add_to_linker::<T, D>(linker, host_getter)?;
            foo::foo::store::add_to_linker_instance::<
                T,
                D,
            >(&mut linker.instance("a")?, host_getter)?;
            Ok(())
        }
        pub fn func_type(&self) -> wasmtime::component::TypedFunc<(), ()> {
            unsafe {
                wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.type_)
            }
        }
        pub async fn call_type<_T, _D>(
            &self,
            accessor: &wasmtime::component::Accessor<_T, _D>,
        ) -> wasmtime::Result<()>
        where
            _T: Send,
            _D: wasmtime::component::HasData,
        {
            let callee = self.func_type();
            let () = callee.call_concurrent(accessor, ()).await?;
            Ok(())
        }
        pub fn func_run(&self) -> wasmtime::component::TypedFunc<(), ()> {
            unsafe { wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.run) }
        }
        pub async fn call_run<_T, _D>(
            &self,
            accessor: &wasmtime::component::Accessor<_T, _D>,
        ) -> wasmtime::Result<()>
        where
            _T: Send,
            _D: wasmtime::component::HasData,
        {
            let callee = self.func_run();
            let () = callee.call_concurrent(accessor, ()).await?;
            Ok(())
        }
        pub fn func_interface4(&self) -> wasmtime::component::TypedFunc<(), ()> {
            unsafe {
                wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.interface4)
            }
        }
        /// The world's struct has a field for each export, named after functions,
        /// and `interface{N}` for interfaces where `N` is their index in the world's
        /// exports, which lists functions first. The field of `%use:%fn/%self` would
        /// therefore be `interface4`, but that's claimed by the function first.
        /// Fields are also loaded into local variables next to `store`.
        pub async fn call_interface4<_T, _D>(
            &self,
            accessor: &wasmtime::component::Accessor<_T, _D>,
        ) -> wasmtime::Result<()>
        where
            _T: Send,
            _D: wasmtime::component::HasData,
        {
            let callee = self.func_interface4();
            let () = callee.call_concurrent(accessor, ()).await?;
            Ok(())
        }
        pub fn func_store(&self) -> wasmtime::component::TypedFunc<(), ()> {
            unsafe {
                wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.store_)
            }
        }
        pub async fn call_store<_T, _D>(
            &self,
            accessor: &wasmtime::component::Accessor<_T, _D>,
        ) -> wasmtime::Result<()>
        where
            _T: Send,
            _D: wasmtime::component::HasData,
        {
            let callee = self.func_store();
            let () = callee.call_concurrent(accessor, ()).await?;
            Ok(())
        }
        pub fn use_fn_self_(&self) -> &exports::use_::fn_::self_::Guest {
            &self.interface4_
        }
        pub fn foo_foo_greeter(&self) -> &exports::foo::foo::greeter::Guest {
            &self.interface5
        }
        pub fn foo(&self) -> &exports::foo_::Guest {
            &self.interface6
        }
        pub fn foo_foo_greeter_(&self) -> &exports::foo_foo_greeter::Guest {
            &self.interface7
        }
        pub fn new_(&self) -> &exports::new::Guest {
            &self.interface8
        }
        pub fn instantiate_(&self) -> &exports::instantiate::Guest {
            &self.interface9
        }
        pub fn instantiate_async_(&self) -> &exports::instantiate_async::Guest {
            &self.interface10
        }
        pub fn add_to_linker_(&self) -> &exports::add_to_linker::Guest {
            &self.interface11
        }
        pub fn add_to_linker_imports_(&self) -> &exports::add_to_linker_imports::Guest {
            &self.interface12
        }
        pub fn call_run_(&self) -> &exports::call_run::Guest {
            &self.interface13
        }
    }
};
pub mod exports_ {
    pub mod bar {
        #[allow(clippy::all)]
        pub mod baz {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
                fn f(
                    host: wasmtime::component::Access<T, Self>,
                ) -> impl ::core::future::Future<Output = ()> + Send;
            }
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
            pub fn add_to_linker_instance<T, D>(
                inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                inst.func_wrap_async(
                    "f",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::f(host).await;
                            Ok(r)
                        })
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, D>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                let mut inst = linker.instance("exports:bar/baz")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
    }
}
pub mod foo {
    pub mod foo {
        #[allow(clippy::all)]
        pub mod store {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
                fn get(
                    host: wasmtime::component::Access<T, Self>,
                ) -> impl ::core::future::Future<Output = u32> + Send;
            }
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
            pub fn add_to_linker_instance<T, D>(
                inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                inst.func_wrap_async(
                    "get",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::get(host).await;
                            Ok((r,))
                        })
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, D>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                let mut inst = linker.instance("foo:foo/store")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
    }
}
pub mod use_ {
    pub mod fn_ {
        #[allow(clippy::all)]
        pub mod self_ {
            #[allow(unused_imports)]
            use wasmtime::component::__internal::Box;
            pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
                fn f(
                    host: wasmtime::component::Access<T, Self>,
                ) -> impl ::core::future::Future<Output = ()> + Send;
            }
            pub trait Host: Send {}
            impl<_T: Host + ?Sized + Send> Host for &mut _T {}
            pub fn add_to_linker_instance<T, D>(
                inst: &mut wasmtime::component::LinkerInstance<'_, T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                inst.func_wrap_async(
                    "f",
                    move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                        wasmtime::component::__internal::Box::new(async move {
                            let access_cx = wasmtime::AsContextMut::as_context_mut(
                                &mut caller,
                            );
                            let host = wasmtime::component::Access::<
                                T,
                                D,
                            >::new(access_cx, host_getter);
                            let r = <D as HostWithStore<T>>::f(host).await;
                            Ok(r)
                        })
                    },
                )?;
                Ok(())
            }
            pub fn add_to_linker<T, D>(
                linker: &mut wasmtime::component::Linker<T>,
                host_getter: fn(&mut T) -> D::Data<'_>,
            ) -> wasmtime::Result<()>
            where
                D: HostWithStore<T>,
                for<'a> D::Data<'a>: Host,
                T: 'static + Send,
            {
                let mut inst = linker.instance("use:fn/self")?;
                add_to_linker_instance::<T, D>(&mut inst, host_getter)
            }
        }
    }
}
#[allow(clippy::all)]
pub mod foo_ {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::Box;
    pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
        fn f(
            host: wasmtime::component::Access<T, Self>,
        ) -> impl ::core::future::Future<Output = ()> + Send;
    }
    pub trait Host: Send {}
    impl<_T: Host + ?Sized + Send> Host for &mut _T {}
    pub fn add_to_linker_instance<T, D>(
        inst: &mut wasmtime::component::LinkerInstance<'_, T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        inst.func_wrap_async(
            "f",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                wasmtime::component::__internal::Box::new(async move {
                    let access_cx = wasmtime::AsContextMut::as_context_mut(&mut caller);
                    let host = wasmtime::component::Access::<
                        T,
                        D,
                    >::new(access_cx, host_getter);
                    let r = <D as HostWithStore<T>>::f(host).await;
                    Ok(r)
                })
            },
        )?;
        Ok(())
    }
    pub fn add_to_linker<T, D>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        let mut inst = linker.instance("foo")?;
        add_to_linker_instance::<T, D>(&mut inst, host_getter)
    }
}
#[allow(clippy::all)]
pub mod exports__ {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::Box;
    pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
        fn f(
            host: wasmtime::component::Access<T, Self>,
        ) -> impl ::core::future::Future<Output = ()> + Send;
    }
    pub trait Host: Send {}
    impl<_T: Host + ?Sized + Send> Host for &mut _T {}
    pub fn add_to_linker_instance<T, D>(
        inst: &mut wasmtime::component::LinkerInstance<'_, T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        inst.func_wrap_async(
            "f",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                wasmtime::component::__internal::Box::new(async move {
                    let access_cx = wasmtime::AsContextMut::as_context_mut(&mut caller);
                    let host = wasmtime::component::Access::<
                        T,
                        D,
                    >::new(access_cx, host_getter);
                    let r = <D as HostWithStore<T>>::f(host).await;
                    Ok(r)
                })
            },
        )?;
        Ok(())
    }
    pub fn add_to_linker<T, D>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        let mut inst = linker.instance("exports")?;
        add_to_linker_instance::<T, D>(&mut inst, host_getter)
    }
}
#[allow(clippy::all)]
pub mod named_imports_ {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::Box;
    pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
        fn f(
            host: wasmtime::component::Access<T, Self>,
        ) -> impl ::core::future::Future<Output = ()> + Send;
    }
    pub trait Host: Send {}
    impl<_T: Host + ?Sized + Send> Host for &mut _T {}
    pub fn add_to_linker_instance<T, D>(
        inst: &mut wasmtime::component::LinkerInstance<'_, T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        inst.func_wrap_async(
            "f",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                wasmtime::component::__internal::Box::new(async move {
                    let access_cx = wasmtime::AsContextMut::as_context_mut(&mut caller);
                    let host = wasmtime::component::Access::<
                        T,
                        D,
                    >::new(access_cx, host_getter);
                    let r = <D as HostWithStore<T>>::f(host).await;
                    Ok(r)
                })
            },
        )?;
        Ok(())
    }
    pub fn add_to_linker<T, D>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        let mut inst = linker.instance("named-imports")?;
        add_to_linker_instance::<T, D>(&mut inst, host_getter)
    }
}
#[allow(clippy::all)]
pub mod type_ {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::Box;
    pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
        fn f(
            host: wasmtime::component::Access<T, Self>,
        ) -> impl ::core::future::Future<Output = ()> + Send;
    }
    pub trait Host: Send {}
    impl<_T: Host + ?Sized + Send> Host for &mut _T {}
    pub fn add_to_linker_instance<T, D>(
        inst: &mut wasmtime::component::LinkerInstance<'_, T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        inst.func_wrap_async(
            "f",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                wasmtime::component::__internal::Box::new(async move {
                    let access_cx = wasmtime::AsContextMut::as_context_mut(&mut caller);
                    let host = wasmtime::component::Access::<
                        T,
                        D,
                    >::new(access_cx, host_getter);
                    let r = <D as HostWithStore<T>>::f(host).await;
                    Ok(r)
                })
            },
        )?;
        Ok(())
    }
    pub fn add_to_linker<T, D>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        let mut inst = linker.instance("type")?;
        add_to_linker_instance::<T, D>(&mut inst, host_getter)
    }
}
#[allow(clippy::all)]
pub mod self_ {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::Box;
    pub trait HostWithStore<T>: wasmtime::component::HasData + Send {
        fn f(
            host: wasmtime::component::Access<T, Self>,
        ) -> impl ::core::future::Future<Output = ()> + Send;
    }
    pub trait Host: Send {}
    impl<_T: Host + ?Sized + Send> Host for &mut _T {}
    pub fn add_to_linker_instance<T, D>(
        inst: &mut wasmtime::component::LinkerInstance<'_, T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        inst.func_wrap_async(
            "f",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                wasmtime::component::__internal::Box::new(async move {
                    let access_cx = wasmtime::AsContextMut::as_context_mut(&mut caller);
                    let host = wasmtime::component::Access::<
                        T,
                        D,
                    >::new(access_cx, host_getter);
                    let r = <D as HostWithStore<T>>::f(host).await;
                    Ok(r)
                })
            },
        )?;
        Ok(())
    }
    pub fn add_to_linker<T, D>(
        linker: &mut wasmtime::component::Linker<T>,
        host_getter: fn(&mut T) -> D::Data<'_>,
    ) -> wasmtime::Result<()>
    where
        D: HostWithStore<T>,
        for<'a> D::Data<'a>: Host,
        T: 'static + Send,
    {
        let mut inst = linker.instance("SELF")?;
        add_to_linker_instance::<T, D>(&mut inst, host_getter)
    }
}
pub mod exports {
    pub mod foo {
        pub mod foo {
            #[allow(clippy::all)]
            pub mod greeter {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::Box;
                #[derive(Clone)]
                pub struct Guest {
                    greet: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    greet: wasmtime::component::ComponentExportIndex,
                }
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new<_T>(
                        _instance_pre: &wasmtime::component::InstancePre<_T>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance = _instance_pre
                            .component()
                            .get_export_index(None, "foo:foo/greeter")
                            .ok_or_else(|| {
                                wasmtime::format_err!(
                                    "no exported instance named `foo:foo/greeter`"
                                )
                            })?;
                        let mut lookup = move |name: &str| {
                            _instance_pre
                                .component()
                                .get_export_index(Some(&instance), name)
                                .ok_or_else(|| {
                                    wasmtime::format_err!(
                                        "instance export `foo:foo/greeter` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let greet = lookup("greet")?;
                        Ok(GuestIndices { greet })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let _instance = instance;
                        let _instance_pre = _instance.instance_pre(&store);
                        let _instance_type = _instance_pre.instance_type();
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let greet = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.greet)?
                            .func();
                        Ok(Guest { greet })
                    }
                }
                impl Guest {
                    pub fn func_greet(&self) -> wasmtime::component::TypedFunc<(), ()> {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.greet)
                        }
                    }
                    pub async fn call_greet<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                    ) -> wasmtime::Result<()>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_greet();
                        let () = callee.call_concurrent(accessor, ()).await?;
                        Ok(())
                    }
                }
            }
        }
    }
    pub mod use_ {
        pub mod fn_ {
            #[allow(clippy::all)]
            pub mod self_ {
                #[allow(unused_imports)]
                use wasmtime::component::__internal::Box;
                #[derive(Clone)]
                pub struct Guest {
                    f: wasmtime::component::Func,
                }
                #[derive(Clone)]
                pub struct GuestIndices {
                    f: wasmtime::component::ComponentExportIndex,
                }
                impl GuestIndices {
                    /// Constructor for [`GuestIndices`] which takes a
                    /// [`Component`](wasmtime::component::Component) as input and can be executed
                    /// before instantiation.
                    ///
                    /// This constructor can be used to front-load string lookups to find exports
                    /// within a component.
                    pub fn new<_T>(
                        _instance_pre: &wasmtime::component::InstancePre<_T>,
                    ) -> wasmtime::Result<GuestIndices> {
                        let instance = _instance_pre
                            .component()
                            .get_export_index(None, "use:fn/self")
                            .ok_or_else(|| {
                                wasmtime::format_err!(
                                    "no exported instance named `use:fn/self`"
                                )
                            })?;
                        let mut lookup = move |name: &str| {
                            _instance_pre
                                .component()
                                .get_export_index(Some(&instance), name)
                                .ok_or_else(|| {
                                    wasmtime::format_err!(
                                        "instance export `use:fn/self` does \
                not have export `{name}`"
                                    )
                                })
                        };
                        let _ = &mut lookup;
                        let f = lookup("f")?;
                        Ok(GuestIndices { f })
                    }
                    pub fn load(
                        &self,
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> wasmtime::Result<Guest> {
                        let _instance = instance;
                        let _instance_pre = _instance.instance_pre(&store);
                        let _instance_type = _instance_pre.instance_type();
                        let mut store = store.as_context_mut();
                        let _ = &mut store;
                        let f = *_instance
                            .get_typed_func::<(), ()>(&mut store, &self.f)?
                            .func();
                        Ok(Guest { f })
                    }
                }
                impl Guest {
                    pub fn func_f(&self) -> wasmtime::component::TypedFunc<(), ()> {
                        unsafe {
                            wasmtime::component::TypedFunc::<
                                (),
                                (),
                            >::new_unchecked(self.f)
                        }
                    }
                    pub async fn call_f<_T, _D>(
                        &self,
                        accessor: &wasmtime::component::Accessor<_T, _D>,
                    ) -> wasmtime::Result<()>
                    where
                        _T: Send,
                        _D: wasmtime::component::HasData,
                    {
                        let callee = self.func_f();
                        let () = callee.call_concurrent(accessor, ()).await?;
                        Ok(())
                    }
                }
            }
        }
    }
    #[allow(clippy::all)]
    pub mod foo_ {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {
            greet: wasmtime::component::Func,
        }
        #[derive(Clone)]
        pub struct GuestIndices {
            greet: wasmtime::component::ComponentExportIndex,
        }
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "foo")
                    .ok_or_else(|| {
                        wasmtime::format_err!("no exported instance named `foo`")
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `foo` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                let greet = lookup("greet")?;
                Ok(GuestIndices { greet })
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                let greet = *_instance
                    .get_typed_func::<(), ()>(&mut store, &self.greet)?
                    .func();
                Ok(Guest { greet })
            }
        }
        impl Guest {
            pub fn func_greet(&self) -> wasmtime::component::TypedFunc<(), ()> {
                unsafe {
                    wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.greet)
                }
            }
            pub async fn call_greet<_T, _D>(
                &self,
                accessor: &wasmtime::component::Accessor<_T, _D>,
            ) -> wasmtime::Result<()>
            where
                _T: Send,
                _D: wasmtime::component::HasData,
            {
                let callee = self.func_greet();
                let () = callee.call_concurrent(accessor, ()).await?;
                Ok(())
            }
        }
    }
    #[allow(clippy::all)]
    pub mod foo_foo_greeter {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {
            greet: wasmtime::component::Func,
        }
        #[derive(Clone)]
        pub struct GuestIndices {
            greet: wasmtime::component::ComponentExportIndex,
        }
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "foo-foo-greeter")
                    .ok_or_else(|| {
                        wasmtime::format_err!(
                            "no exported instance named `foo-foo-greeter`"
                        )
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `foo-foo-greeter` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                let greet = lookup("greet")?;
                Ok(GuestIndices { greet })
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                let greet = *_instance
                    .get_typed_func::<(), ()>(&mut store, &self.greet)?
                    .func();
                Ok(Guest { greet })
            }
        }
        impl Guest {
            pub fn func_greet(&self) -> wasmtime::component::TypedFunc<(), ()> {
                unsafe {
                    wasmtime::component::TypedFunc::<(), ()>::new_unchecked(self.greet)
                }
            }
            pub async fn call_greet<_T, _D>(
                &self,
                accessor: &wasmtime::component::Accessor<_T, _D>,
            ) -> wasmtime::Result<()>
            where
                _T: Send,
                _D: wasmtime::component::HasData,
            {
                let callee = self.func_greet();
                let () = callee.call_concurrent(accessor, ()).await?;
                Ok(())
            }
        }
    }
    #[allow(clippy::all)]
    pub mod new {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {}
        #[derive(Clone)]
        pub struct GuestIndices {}
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "new")
                    .ok_or_else(|| {
                        wasmtime::format_err!("no exported instance named `new`")
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `new` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                Ok(GuestIndices {})
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                Ok(Guest {})
            }
        }
        impl Guest {}
    }
    #[allow(clippy::all)]
    pub mod instantiate {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {}
        #[derive(Clone)]
        pub struct GuestIndices {}
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "instantiate")
                    .ok_or_else(|| {
                        wasmtime::format_err!("no exported instance named `instantiate`")
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `instantiate` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                Ok(GuestIndices {})
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                Ok(Guest {})
            }
        }
        impl Guest {}
    }
    #[allow(clippy::all)]
    pub mod instantiate_async {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {}
        #[derive(Clone)]
        pub struct GuestIndices {}
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "instantiate-async")
                    .ok_or_else(|| {
                        wasmtime::format_err!(
                            "no exported instance named `instantiate-async`"
                        )
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `instantiate-async` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                Ok(GuestIndices {})
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                Ok(Guest {})
            }
        }
        impl Guest {}
    }
    #[allow(clippy::all)]
    pub mod add_to_linker {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {}
        #[derive(Clone)]
        pub struct GuestIndices {}
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "add-to-linker")
                    .ok_or_else(|| {
                        wasmtime::format_err!(
                            "no exported instance named `add-to-linker`"
                        )
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `add-to-linker` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                Ok(GuestIndices {})
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                Ok(Guest {})
            }
        }
        impl Guest {}
    }
    #[allow(clippy::all)]
    pub mod add_to_linker_imports {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {}
        #[derive(Clone)]
        pub struct GuestIndices {}
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "add-to-linker-imports")
                    .ok_or_else(|| {
                        wasmtime::format_err!(
                            "no exported instance named `add-to-linker-imports`"
                        )
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `add-to-linker-imports` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                Ok(GuestIndices {})
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                Ok(Guest {})
            }
        }
        impl Guest {}
    }
    #[allow(clippy::all)]
    pub mod call_run {
        #[allow(unused_imports)]
        use wasmtime::component::__internal::Box;
        #[derive(Clone)]
        pub struct Guest {}
        #[derive(Clone)]
        pub struct GuestIndices {}
        impl GuestIndices {
            /// Constructor for [`GuestIndices`] which takes a
            /// [`Component`](wasmtime::component::Component) as input and can be executed
            /// before instantiation.
            ///
            /// This constructor can be used to front-load string lookups to find exports
            /// within a component.
            pub fn new<_T>(
                _instance_pre: &wasmtime::component::InstancePre<_T>,
            ) -> wasmtime::Result<GuestIndices> {
                let instance = _instance_pre
                    .component()
                    .get_export_index(None, "call-run")
                    .ok_or_else(|| {
                        wasmtime::format_err!("no exported instance named `call-run`")
                    })?;
                let mut lookup = move |name: &str| {
                    _instance_pre
                        .component()
                        .get_export_index(Some(&instance), name)
                        .ok_or_else(|| {
                            wasmtime::format_err!(
                                "instance export `call-run` does \
            not have export `{name}`"
                            )
                        })
                };
                let _ = &mut lookup;
                Ok(GuestIndices {})
            }
            pub fn load(
                &self,
                mut store: impl wasmtime::AsContextMut,
                instance: &wasmtime::component::Instance,
            ) -> wasmtime::Result<Guest> {
                let _instance = instance;
                let _instance_pre = _instance.instance_pre(&store);
                let _instance_type = _instance_pre.instance_type();
                let mut store = store.as_context_mut();
                let _ = &mut store;
                Ok(Guest {})
            }
        }
        impl Guest {}
    }
}
