use crate::component::func::HostFunc;
use crate::component::matching::InstanceType;
use crate::component::{Component, ComponentNamedList, Func, Lift, Lower, ResourceType, TypedFunc};
use crate::instance::OwnedImports;
use crate::linker::DefinitionType;
use crate::runtime::vm::component::{ComponentInstance, OwnedComponentInstance};
use crate::runtime::vm::VMFuncRef;
use crate::store::{StoreOpaque, Stored};
use crate::{AsContextMut, Module, StoreContextMut};
use anyhow::{anyhow, Context, Result};
use std::marker;
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::prelude::IndexMap;
use wasmtime_environ::{component::*, EngineOrModuleTypeIndex};
use wasmtime_environ::{EntityIndex, EntityType, Global, PrimaryMap, WasmValType};

/// An instantiated component.
///
/// This type represents an instantiated [`Component`](super::Component).
/// Instances have exports which can be accessed through functions such as
/// [`Instance::get_func`] or [`Instance::exports`]. Instances are owned by a
/// [`Store`](crate::Store) and all methods require a handle to the store.
///
/// Component instances are created through
/// [`Linker::instantiate`](super::Linker::instantiate) and its family of
/// methods.
///
/// This type is similar to the core wasm version
/// [`wasmtime::Instance`](crate::Instance) except that it represents an
/// instantiated component instead of an instantiated module.
#[derive(Copy, Clone)]
pub struct Instance(pub(crate) Stored<Option<Box<InstanceData>>>);

pub(crate) struct InstanceData {
    instances: PrimaryMap<RuntimeInstanceIndex, crate::Instance>,

    // NB: in the future if necessary it would be possible to avoid storing an
    // entire `Component` here and instead storing only information such as:
    //
    // * Some reference to `Arc<ComponentTypes>`
    // * Necessary references to closed-over modules which are exported from the
    //   component itself.
    //
    // Otherwise the full guts of this component should only ever be used during
    // the instantiation of this instance, meaning that after instantiation much
    // of the component can be thrown away (theoretically).
    component: Component,

    state: OwnedComponentInstance,

    /// Arguments that this instance used to be instantiated.
    ///
    /// Strong references are stored to these arguments since pointers are saved
    /// into the structures such as functions within the
    /// `OwnedComponentInstance` but it's our job to keep them alive.
    ///
    /// One purpose of this storage is to enable embedders to drop a `Linker`,
    /// for example, after a component is instantiated. In that situation if the
    /// arguments weren't held here then they might be dropped, and structures
    /// such as `.lowering()` which point back into the original function would
    /// become stale and use-after-free conditions when used. By preserving the
    /// entire list here though we're guaranteed that nothing is lost for the
    /// duration of the lifetime of this instance.
    imports: Arc<PrimaryMap<RuntimeImportIndex, RuntimeImport>>,
}

impl Instance {
    /// Returns information about the exports of this instance.
    ///
    /// This method can be used to extract exported values from this component
    /// instance. The argument to this method be a handle to the store that
    /// this instance was instantiated into.
    ///
    /// The returned [`Exports`] value can be used to lookup exported items by
    /// name.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn exports<'a, T: 'a>(&self, store: impl Into<StoreContextMut<'a, T>>) -> Exports<'a> {
        let store = store.into();
        Exports::new(store.0, self)
    }

    /// Looks up a function by name within this [`Instance`].
    ///
    /// This is a convenience method for calling [`Instance::exports`] followed
    /// by [`ExportInstance::func`].
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_func(&self, mut store: impl AsContextMut, name: &str) -> Option<Func> {
        self.exports(store.as_context_mut()).root().func(name)
    }

    /// Looks up an exported [`Func`] value by name and with its type.
    ///
    /// This function is a convenience wrapper over [`Instance::get_func`] and
    /// [`Func::typed`]. For more information see the linked documentation.
    ///
    /// Returns an error if `name` isn't a function export or if the export's
    /// type did not match `Params` or `Results`
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_typed_func<Params, Results>(
        &self,
        mut store: impl AsContextMut,
        name: &str,
    ) -> Result<TypedFunc<Params, Results>>
    where
        Params: ComponentNamedList + Lower,
        Results: ComponentNamedList + Lift,
    {
        let f = self
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(f.typed::<Params, Results>(store)
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
    }

    /// Looks up a module by name within this [`Instance`].
    ///
    /// The `store` specified must be the store that this instance lives within
    /// and `name` is the name of the function to lookup. If the module is
    /// found `Some` is returned otherwise `None` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_module(&self, mut store: impl AsContextMut, name: &str) -> Option<Module> {
        self.exports(store.as_context_mut())
            .root()
            .module(name)
            .cloned()
    }

    /// Looks up an exported resource type by name within this [`Instance`].
    ///
    /// The `store` specified must be the store that this instance lives within
    /// and `name` is the name of the function to lookup. If the resource type
    /// is found `Some` is returned otherwise `None` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_resource(&self, mut store: impl AsContextMut, name: &str) -> Option<ResourceType> {
        self.exports(store.as_context_mut()).root().resource(name)
    }
}

impl InstanceData {
    pub fn lookup_def(&self, store: &mut StoreOpaque, def: &CoreDef) -> crate::runtime::vm::Export {
        match def {
            CoreDef::Export(e) => self.lookup_export(store, e),
            CoreDef::Trampoline(idx) => {
                crate::runtime::vm::Export::Function(crate::runtime::vm::ExportFunction {
                    func_ref: self.state.trampoline_func_ref(*idx),
                })
            }
            CoreDef::InstanceFlags(idx) => {
                crate::runtime::vm::Export::Global(crate::runtime::vm::ExportGlobal {
                    definition: self.state.instance_flags(*idx).as_raw(),
                    vmctx: std::ptr::null_mut(),
                    global: Global {
                        wasm_ty: WasmValType::I32,
                        mutability: true,
                    },
                })
            }
        }
    }

    pub fn lookup_export<T>(
        &self,
        store: &mut StoreOpaque,
        item: &CoreExport<T>,
    ) -> crate::runtime::vm::Export
    where
        T: Copy + Into<EntityIndex>,
    {
        let instance = &self.instances[item.instance];
        let id = instance.id(store);
        let instance = store.instance_mut(id);
        let idx = match &item.item {
            ExportItem::Index(idx) => (*idx).into(),

            // FIXME: ideally at runtime we don't actually do any name lookups
            // here. This will only happen when the host supplies an imported
            // module so while the structure can't be known at compile time we
            // do know at `InstancePre` time, for example, what all the host
            // imports are. In theory we should be able to, as part of
            // `InstancePre` construction, perform all name=>index mappings
            // during that phase so the actual instantiation of an `InstancePre`
            // skips all string lookups. This should probably only be
            // investigated if this becomes a performance issue though.
            ExportItem::Name(name) => instance.module().exports[name],
        };
        instance.get_export_by_index(idx)
    }

    #[inline]
    pub fn instance(&self) -> &ComponentInstance {
        &self.state
    }

    #[inline]
    pub fn instance_ptr(&self) -> *mut ComponentInstance {
        self.state.instance_ptr()
    }

    #[inline]
    pub fn component_types(&self) -> &Arc<ComponentTypes> {
        self.component.types()
    }

    #[inline]
    pub fn ty(&self) -> InstanceType<'_> {
        InstanceType::new(self.instance())
    }

    // NB: This method is only intended to be called during the instantiation
    // process because the `Arc::get_mut` here is fallible and won't generally
    // succeed once the instance has been handed to the embedder. Before that
    // though it should be guaranteed that the single owning reference currently
    // lives within the `ComponentInstance` that's being built.
    fn resource_types_mut(&mut self) -> &mut ImportedResources {
        Arc::get_mut(self.state.resource_types_mut())
            .unwrap()
            .downcast_mut()
            .unwrap()
    }
}

struct Instantiator<'a> {
    component: &'a Component,
    data: InstanceData,
    core_imports: OwnedImports,
    imports: &'a PrimaryMap<RuntimeImportIndex, RuntimeImport>,
}

pub(crate) enum RuntimeImport {
    Func(Arc<HostFunc>),
    Module(Module),
    Resource {
        ty: ResourceType,

        // A strong reference to the host function that represents the
        // destructor for this resource. At this time all resources here are
        // host-defined resources. Note that this is itself never read because
        // the funcref below points to it.
        //
        // Also note that the `Arc` here is used to support the same host
        // function being used across multiple instances simultaneously. Or
        // otherwise this makes `InstancePre::instantiate` possible to create
        // separate instances all sharing the same host function.
        _dtor: Arc<crate::func::HostFunc>,

        // A raw function which is filled out (including `wasm_call`) which
        // points to the internals of the `_dtor` field. This is read and
        // possibly executed by wasm.
        dtor_funcref: VMFuncRef,
    },
}

pub type ImportedResources = PrimaryMap<ResourceIndex, ResourceType>;

impl<'a> Instantiator<'a> {
    fn new(
        component: &'a Component,
        store: &mut StoreOpaque,
        imports: &'a Arc<PrimaryMap<RuntimeImportIndex, RuntimeImport>>,
    ) -> Instantiator<'a> {
        let env_component = component.env_component();
        store.modules_mut().register_component(component);
        let imported_resources: ImportedResources =
            PrimaryMap::with_capacity(env_component.imported_resources.len());
        Instantiator {
            component,
            imports,
            core_imports: OwnedImports::empty(),
            data: InstanceData {
                instances: PrimaryMap::with_capacity(env_component.num_runtime_instances as usize),
                component: component.clone(),
                state: OwnedComponentInstance::new(
                    component.runtime_info(),
                    Arc::new(imported_resources),
                    store.traitobj(),
                ),
                imports: imports.clone(),
            },
        }
    }

    fn run<T>(&mut self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        let env_component = self.component.env_component();

        // Before all initializers are processed configure all destructors for
        // host-defined resources. No initializer will correspond to these and
        // it's required to happen before they're needed, so execute this first.
        for (idx, import) in env_component.imported_resources.iter() {
            let (ty, func_ref) = match &self.imports[*import] {
                RuntimeImport::Resource {
                    ty, dtor_funcref, ..
                } => (*ty, NonNull::from(dtor_funcref)),
                _ => unreachable!(),
            };
            let i = self.data.resource_types_mut().push(ty);
            assert_eq!(i, idx);
            self.data.state.set_resource_destructor(idx, Some(func_ref));
        }

        // Next configure all `VMFuncRef`s for trampolines that this component
        // will require. These functions won't actually get used until their
        // associated state has been initialized through the global initializers
        // below, but the funcrefs can all be configured here.
        for (idx, sig) in env_component.trampolines.iter() {
            let ptrs = self.component.trampoline_ptrs(idx);
            let signature = match self.component.signatures().shared_type(*sig) {
                Some(s) => s,
                None => panic!("found unregistered signature: {sig:?}"),
            };

            self.data.state.set_trampoline(
                idx,
                ptrs.wasm_call,
                ptrs.native_call,
                ptrs.array_call,
                signature,
            );
        }

        for initializer in env_component.initializers.iter() {
            match initializer {
                GlobalInitializer::InstantiateModule(m) => {
                    let module;
                    let imports = match m {
                        // Since upvars are statically know we know that the
                        // `args` list is already in the right order.
                        InstantiateModule::Static(idx, args) => {
                            module = self.component.static_module(*idx);
                            self.build_imports(store.0, module, args.iter())
                        }

                        // With imports, unlike upvars, we need to do runtime
                        // lookups with strings to determine the order of the
                        // imports since it's whatever the actual module
                        // requires.
                        //
                        // FIXME: see the note in `ExportItem::Name` handling
                        // above for how we ideally shouldn't do string lookup
                        // here.
                        InstantiateModule::Import(idx, args) => {
                            module = match &self.imports[*idx] {
                                RuntimeImport::Module(m) => m,
                                _ => unreachable!(),
                            };
                            let args = module
                                .imports()
                                .map(|import| &args[import.module()][import.name()]);
                            self.build_imports(store.0, module, args)
                        }
                    };

                    // Note that the unsafety here should be ok because the
                    // validity of the component means that type-checks have
                    // already been performed. This means that the unsafety due
                    // to imports having the wrong type should not happen here.
                    //
                    // Also note we are calling new_started_impl because we have
                    // already checked for asyncness and are running on a fiber
                    // if required.

                    let i = unsafe {
                        crate::Instance::new_started_impl(store, module, imports.as_ref())?
                    };
                    self.data.instances.push(i);
                }

                GlobalInitializer::LowerImport { import, index } => {
                    let func = match &self.imports[*import] {
                        RuntimeImport::Func(func) => func,
                        _ => unreachable!(),
                    };
                    self.data.state.set_lowering(*index, func.lowering());
                }

                GlobalInitializer::ExtractMemory(mem) => self.extract_memory(store.0, mem),

                GlobalInitializer::ExtractRealloc(realloc) => {
                    self.extract_realloc(store.0, realloc)
                }

                GlobalInitializer::ExtractPostReturn(post_return) => {
                    self.extract_post_return(store.0, post_return)
                }

                GlobalInitializer::Resource(r) => self.resource(store.0, r),
            }
        }
        Ok(())
    }

    fn resource(&mut self, store: &mut StoreOpaque, resource: &Resource) {
        let dtor = resource
            .dtor
            .as_ref()
            .map(|dtor| self.data.lookup_def(store, dtor));
        let dtor = dtor.map(|export| match export {
            crate::runtime::vm::Export::Function(f) => f.func_ref,
            _ => unreachable!(),
        });
        let index = self
            .component
            .env_component()
            .resource_index(resource.index);
        self.data.state.set_resource_destructor(index, dtor);
        let ty = ResourceType::guest(store.id(), &self.data.state, resource.index);
        let i = self.data.resource_types_mut().push(ty);
        debug_assert_eq!(i, index);
    }

    fn extract_memory(&mut self, store: &mut StoreOpaque, memory: &ExtractMemory) {
        let mem = match self.data.lookup_export(store, &memory.export) {
            crate::runtime::vm::Export::Memory(m) => m,
            _ => unreachable!(),
        };
        self.data
            .state
            .set_runtime_memory(memory.index, mem.definition);
    }

    fn extract_realloc(&mut self, store: &mut StoreOpaque, realloc: &ExtractRealloc) {
        let func_ref = match self.data.lookup_def(store, &realloc.def) {
            crate::runtime::vm::Export::Function(f) => f.func_ref,
            _ => unreachable!(),
        };
        self.data.state.set_runtime_realloc(realloc.index, func_ref);
    }

    fn extract_post_return(&mut self, store: &mut StoreOpaque, post_return: &ExtractPostReturn) {
        let func_ref = match self.data.lookup_def(store, &post_return.def) {
            crate::runtime::vm::Export::Function(f) => f.func_ref,
            _ => unreachable!(),
        };
        self.data
            .state
            .set_runtime_post_return(post_return.index, func_ref);
    }

    fn build_imports<'b>(
        &mut self,
        store: &mut StoreOpaque,
        module: &Module,
        args: impl Iterator<Item = &'b CoreDef>,
    ) -> &OwnedImports {
        self.core_imports.clear();
        self.core_imports.reserve(module);
        let mut imports = module.compiled_module().module().imports();

        for arg in args {
            // The general idea of Wasmtime is that at runtime type-checks for
            // core wasm instantiations internally within a component are
            // unnecessary and superfluous. Naturally though mistakes may be
            // made, so double-check this property of wasmtime in debug mode.

            if cfg!(debug_assertions) {
                let (imp_module, imp_name, expected) = imports.next().unwrap();
                self.assert_type_matches(store, module, arg, imp_module, imp_name, expected);
            }

            // The unsafety here should be ok since the `export` is loaded
            // directly from an instance which should only give us valid export
            // items.
            let export = self.data.lookup_def(store, arg);
            unsafe {
                self.core_imports.push_export(&export);
            }
        }
        debug_assert!(imports.next().is_none());

        &self.core_imports
    }

    fn assert_type_matches(
        &self,
        store: &mut StoreOpaque,
        module: &Module,
        arg: &CoreDef,
        imp_module: &str,
        imp_name: &str,
        expected: EntityType,
    ) {
        let export = self.data.lookup_def(store, arg);

        // If this value is a core wasm function then the type check is inlined
        // here. This can otherwise fail `Extern::from_wasmtime_export` because
        // there's no guarantee that there exists a trampoline for `f` so this
        // can't fall through to the case below
        if let crate::runtime::vm::Export::Function(f) = &export {
            let expected = match expected.unwrap_func() {
                EngineOrModuleTypeIndex::Engine(e) => Some(e),
                EngineOrModuleTypeIndex::Module(m) => module.signatures().shared_type(m),
                EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
            };
            let actual = unsafe { f.func_ref.as_ref().type_index };
            assert_eq!(
                expected,
                Some(actual),
                "type mismatch for import {imp_module:?} {imp_name:?}!!!\n\n\
                 expected {:#?}\n\n\
                 found {:#?}",
                expected.and_then(|e| store.engine().signatures().borrow(e)),
                store.engine().signatures().borrow(actual)
            );
            return;
        }

        let val = unsafe { crate::Extern::from_wasmtime_export(export, store) };
        let ty = DefinitionType::from(store, &val);
        crate::types::matching::MatchCx::new(module.engine())
            .definition(&expected, &ty)
            .expect("unexpected typecheck failure");
    }
}

/// A "pre-instantiated" [`Instance`] which has all of its arguments already
/// supplied and is ready to instantiate.
///
/// This structure represents an efficient form of instantiation where import
/// type-checking and import lookup has all been resolved by the time that this
/// type is created. This type is primarily created through the
/// [`Linker::instantiate_pre`](crate::component::Linker::instantiate_pre)
/// method.
pub struct InstancePre<T> {
    component: Component,
    imports: Arc<PrimaryMap<RuntimeImportIndex, RuntimeImport>>,
    _marker: marker::PhantomData<fn() -> T>,
}

// `InstancePre`'s clone does not require `T: Clone`
impl<T> Clone for InstancePre<T> {
    fn clone(&self) -> Self {
        Self {
            component: self.component.clone(),
            imports: self.imports.clone(),
            _marker: self._marker,
        }
    }
}

impl<T> InstancePre<T> {
    /// This function is `unsafe` since there's no guarantee that the
    /// `RuntimeImport` items provided are guaranteed to work with the `T` of
    /// the store.
    ///
    /// Additionally there is no static guarantee that the `imports` provided
    /// satisfy the imports of the `component` provided.
    pub(crate) unsafe fn new_unchecked(
        component: Component,
        imports: PrimaryMap<RuntimeImportIndex, RuntimeImport>,
    ) -> InstancePre<T> {
        InstancePre {
            component,
            imports: Arc::new(imports),
            _marker: marker::PhantomData,
        }
    }

    /// Returns the underlying component that will be instantiated.
    pub fn component(&self) -> &Component {
        &self.component
    }

    /// Performs the instantiation process into the store specified.
    //
    // TODO: needs more docs
    pub fn instantiate(&self, store: impl AsContextMut<Data = T>) -> Result<Instance> {
        assert!(
            !store.as_context().async_support(),
            "must use async instantiation when async support is enabled"
        );
        self.instantiate_impl(store)
    }
    /// Performs the instantiation process into the store specified.
    ///
    /// Exactly like [`Self::instantiate`] except for use on async stores.
    //
    // TODO: needs more docs
    #[cfg(feature = "async")]
    pub async fn instantiate_async(
        &self,
        mut store: impl AsContextMut<Data = T>,
    ) -> Result<Instance>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "must use sync instantiation when async support is disabled"
        );
        store.on_fiber(|store| self.instantiate_impl(store)).await?
    }

    fn instantiate_impl(&self, mut store: impl AsContextMut<Data = T>) -> Result<Instance> {
        let mut store = store.as_context_mut();
        store
            .engine()
            .allocator()
            .increment_component_instance_count()?;
        let mut instantiator = Instantiator::new(&self.component, store.0, &self.imports);
        instantiator.run(&mut store).map_err(|e| {
            store
                .engine()
                .allocator()
                .decrement_component_instance_count();
            e
        })?;
        let data = Box::new(instantiator.data);
        let instance = Instance(store.0.store_data_mut().insert(Some(data)));
        store.0.push_component_instance(instance);
        Ok(instance)
    }
}

/// Description of the exports of an [`Instance`].
///
/// This structure is created through the [`Instance::exports`] method and is
/// used lookup exports by name from within an instance.
pub struct Exports<'store> {
    store: &'store mut StoreOpaque,
    data: Option<Box<InstanceData>>,
    instance: Instance,
}

impl<'store> Exports<'store> {
    fn new(store: &'store mut StoreOpaque, instance: &Instance) -> Exports<'store> {
        // Note that the `InstanceData` is `take`n from the store here. That's
        // to ease with the various liftimes in play here where we often need
        // simultaneous borrows into the `store` and the `data`.
        //
        // To put the data back into the store the `Drop for Exports<'_>` will
        // restore the state of the world.
        Exports {
            data: store[instance.0].take(),
            store,
            instance: *instance,
        }
    }

    /// Returns the "root" instance of this set of exports, or the items that
    /// are directly exported from the instance that this was created from.
    pub fn root(&mut self) -> ExportInstance<'_, '_> {
        let data = self.data.as_ref().unwrap();
        ExportInstance {
            exports: &data.component.env_component().exports,
            instance: &self.instance,
            data,
            store: self.store,
        }
    }

    /// Returns the items that the named instance exports.
    ///
    /// This method will lookup the exported instance with the name `name` from
    /// this list of exports and return a descriptin of that instance's
    /// exports.
    pub fn instance(&mut self, name: &str) -> Option<ExportInstance<'_, '_>> {
        self.root().into_instance(name)
    }

    // FIXME: should all the func/module/typed_func methods below be mirrored
    // here as well? They're already mirrored on `Instance` and otherwise
    // this is attempting to look like the `Linker` API "but in reverse"
    // somewhat.
}

impl Drop for Exports<'_> {
    fn drop(&mut self) {
        // See `Exports::new` for where this data was originally extracted, and
        // this is just restoring the state of the world.
        self.store[self.instance.0] = self.data.take();
    }
}

/// Description of the exports of a single instance.
///
/// This structure is created from [`Exports`] via the [`Exports::root`] or
/// [`Exports::instance`] methods. This type provides access to the first layer
/// of exports within an instance. The [`ExportInstance::instance`] method
/// can be used to provide nested access to sub-instances.
pub struct ExportInstance<'a, 'store> {
    exports: &'a IndexMap<String, Export>,
    instance: &'a Instance,
    data: &'a InstanceData,
    store: &'store mut StoreOpaque,
}

impl<'a, 'store> ExportInstance<'a, 'store> {
    /// Same as [`Instance::get_func`]
    pub fn func(&mut self, name: &str) -> Option<Func> {
        match self.exports.get(name)? {
            Export::LiftedFunction { ty, func, options } => Some(Func::from_lifted_func(
                self.store,
                self.instance,
                self.data,
                *ty,
                func,
                options,
            )),
            Export::ModuleStatic(_)
            | Export::ModuleImport { .. }
            | Export::Instance { .. }
            | Export::Type(_) => None,
        }
    }

    /// Same as [`Instance::get_typed_func`]
    pub fn typed_func<Params, Results>(&mut self, name: &str) -> Result<TypedFunc<Params, Results>>
    where
        Params: ComponentNamedList + Lower,
        Results: ComponentNamedList + Lift,
    {
        let func = self
            .func(name)
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(func
            ._typed::<Params, Results>(self.store, Some(self.data))
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
    }

    /// Same as [`Instance::get_module`]
    pub fn module(&mut self, name: &str) -> Option<&'a Module> {
        match self.exports.get(name)? {
            Export::ModuleStatic(idx) => Some(&self.data.component.static_module(*idx)),
            Export::ModuleImport { import, .. } => Some(match &self.data.imports[*import] {
                RuntimeImport::Module(m) => m,
                _ => unreachable!(),
            }),
            _ => None,
        }
    }

    /// Same as [`Instance::get_resource`]
    pub fn resource(&mut self, name: &str) -> Option<ResourceType> {
        match self.exports.get(name)? {
            Export::Type(TypeDef::Resource(id)) => Some(self.data.ty().resource_type(*id)),
            Export::Type(_)
            | Export::LiftedFunction { .. }
            | Export::ModuleStatic(_)
            | Export::ModuleImport { .. }
            | Export::Instance { .. } => None,
        }
    }

    /// Returns an iterator of all of the exported modules that this instance
    /// contains.
    //
    // FIXME: this should probably be generalized in some form to something else
    // that either looks like:
    //
    // * an iterator over all exports
    // * an iterator for a `Component` with type information followed by a
    //   `get_module` function here
    //
    // For now this is just quick-and-dirty to get wast support for iterating
    // over exported modules to work.
    pub fn modules(&self) -> impl Iterator<Item = (&'a str, &'a Module)> + '_ {
        self.exports.iter().filter_map(|(name, export)| {
            let module = match *export {
                Export::ModuleStatic(idx) => self.data.component.static_module(idx),
                Export::ModuleImport { import, .. } => match &self.data.imports[import] {
                    RuntimeImport::Module(m) => m,
                    _ => unreachable!(),
                },
                _ => return None,
            };
            Some((name.as_str(), module))
        })
    }

    fn as_mut(&mut self) -> ExportInstance<'a, '_> {
        ExportInstance {
            exports: self.exports,
            instance: self.instance,
            data: self.data,
            store: self.store,
        }
    }

    /// Looks up the exported instance with the `name` specified and returns
    /// a description of its exports.
    pub fn instance(&mut self, name: &str) -> Option<ExportInstance<'a, '_>> {
        self.as_mut().into_instance(name)
    }

    /// Same as [`ExportInstance::instance`] but consumes self to yield a
    /// return value with the same lifetimes.
    pub fn into_instance(self, name: &str) -> Option<ExportInstance<'a, 'store>> {
        match self.exports.get(name)? {
            Export::Instance { exports, .. } => Some(ExportInstance {
                exports,
                instance: self.instance,
                data: self.data,
                store: self.store,
            }),
            _ => None,
        }
    }
}
