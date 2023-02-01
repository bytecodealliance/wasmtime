use crate::component::func::HostFunc;
use crate::component::{Component, ComponentNamedList, Func, Lift, Lower, TypedFunc};
use crate::instance::OwnedImports;
use crate::linker::DefinitionType;
use crate::store::{StoreOpaque, Stored};
use crate::{AsContextMut, Module, StoreContextMut};
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use std::marker;
use std::sync::Arc;
use wasmtime_environ::component::{
    AlwaysTrap, ComponentTypes, CoreDef, CoreExport, Export, ExportItem, ExtractMemory,
    ExtractPostReturn, ExtractRealloc, GlobalInitializer, InstantiateModule, LowerImport,
    RuntimeImportIndex, RuntimeInstanceIndex, RuntimeModuleIndex, Transcoder,
};
use wasmtime_environ::{EntityIndex, EntityType, Global, GlobalInit, PrimaryMap, WasmType};
use wasmtime_runtime::component::{ComponentInstance, OwnedComponentInstance};

/// An instantiated component.
///
/// This is similar to [`crate::Instance`] except that it represents an
/// instantiated component instead of an instantiated module. Otherwise though
/// the two behave similarly.
//
// FIXME: need to write more docs here.
#[derive(Copy, Clone)]
pub struct Instance(pub(crate) Stored<Option<Box<InstanceData>>>);

pub(crate) struct InstanceData {
    instances: PrimaryMap<RuntimeInstanceIndex, crate::Instance>,
    // FIXME: shouldn't store the entire component here which keeps upvars
    // alive and things like that, instead only the bare minimum necessary
    // should be kept alive here (mostly just `wasmtime_environ::Component`).
    component: Component,
    exported_modules: PrimaryMap<RuntimeModuleIndex, Module>,

    state: OwnedComponentInstance,

    /// Functions that this instance used during instantiation.
    ///
    /// Strong references are stored to these functions since pointers are saved
    /// into the functions within the `OwnedComponentInstance` but it's our job
    /// to keep them alive.
    funcs: Vec<Arc<HostFunc>>,
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
    pub fn get_typed_func<Params, Results, S>(
        &self,
        mut store: S,
        name: &str,
    ) -> Result<TypedFunc<Params, Results>>
    where
        Params: ComponentNamedList + Lower,
        Results: ComponentNamedList + Lift,
        S: AsContextMut,
    {
        let f = self
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(f.typed::<Params, Results, _>(store)
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
    }

    /// Looks up a module by name within this [`Instance`].
    ///
    /// The `store` specified must be the store that this instance lives within
    /// and `name` is the name of the function to lookup. If the function is
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
}

impl InstanceData {
    pub fn lookup_def(&self, store: &mut StoreOpaque, def: &CoreDef) -> wasmtime_runtime::Export {
        match def {
            CoreDef::Export(e) => self.lookup_export(store, e),
            CoreDef::Lowered(idx) => {
                wasmtime_runtime::Export::Function(wasmtime_runtime::ExportFunction {
                    anyfunc: self.state.lowering_anyfunc(*idx),
                })
            }
            CoreDef::AlwaysTrap(idx) => {
                wasmtime_runtime::Export::Function(wasmtime_runtime::ExportFunction {
                    anyfunc: self.state.always_trap_anyfunc(*idx),
                })
            }
            CoreDef::InstanceFlags(idx) => {
                wasmtime_runtime::Export::Global(wasmtime_runtime::ExportGlobal {
                    definition: self.state.instance_flags(*idx).as_raw(),
                    global: Global {
                        wasm_ty: WasmType::I32,
                        mutability: true,
                        initializer: GlobalInit::I32Const(0),
                    },
                })
            }
            CoreDef::Transcoder(idx) => {
                wasmtime_runtime::Export::Function(wasmtime_runtime::ExportFunction {
                    anyfunc: self.state.transcoder_anyfunc(*idx),
                })
            }
        }
    }

    pub fn lookup_export<T>(
        &self,
        store: &mut StoreOpaque,
        item: &CoreExport<T>,
    ) -> wasmtime_runtime::Export
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

    pub fn instance(&self) -> &ComponentInstance {
        &self.state
    }

    pub fn component_types(&self) -> &Arc<ComponentTypes> {
        self.component.types()
    }
}

struct Instantiator<'a> {
    component: &'a Component,
    data: InstanceData,
    core_imports: OwnedImports,
    imports: &'a PrimaryMap<RuntimeImportIndex, RuntimeImport>,
}

pub enum RuntimeImport {
    Func(Arc<HostFunc>),
    Module(Module),
}

impl<'a> Instantiator<'a> {
    fn new(
        component: &'a Component,
        store: &mut StoreOpaque,
        imports: &'a PrimaryMap<RuntimeImportIndex, RuntimeImport>,
    ) -> Instantiator<'a> {
        let env_component = component.env_component();
        store.modules_mut().register_component(component);
        Instantiator {
            component,
            imports,
            core_imports: OwnedImports::empty(),
            data: InstanceData {
                instances: PrimaryMap::with_capacity(env_component.num_runtime_instances as usize),
                component: component.clone(),
                exported_modules: PrimaryMap::with_capacity(
                    env_component.num_runtime_modules as usize,
                ),
                state: OwnedComponentInstance::new(env_component, store.traitobj()),
                funcs: Vec::new(),
            },
        }
    }

    fn run<T>(&mut self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        let env_component = self.component.env_component();
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

                GlobalInitializer::LowerImport(import) => self.lower_import(import),

                GlobalInitializer::AlwaysTrap(trap) => self.always_trap(trap),

                GlobalInitializer::ExtractMemory(mem) => self.extract_memory(store.0, mem),

                GlobalInitializer::ExtractRealloc(realloc) => {
                    self.extract_realloc(store.0, realloc)
                }

                GlobalInitializer::ExtractPostReturn(post_return) => {
                    self.extract_post_return(store.0, post_return)
                }

                GlobalInitializer::SaveStaticModule(idx) => {
                    self.data
                        .exported_modules
                        .push(self.component.static_module(*idx).clone());
                }

                GlobalInitializer::SaveModuleImport(idx) => {
                    self.data.exported_modules.push(match &self.imports[*idx] {
                        RuntimeImport::Module(m) => m.clone(),
                        _ => unreachable!(),
                    });
                }

                GlobalInitializer::Transcoder(e) => self.transcoder(e),
            }
        }
        Ok(())
    }

    fn lower_import(&mut self, import: &LowerImport) {
        let func = match &self.imports[import.import] {
            RuntimeImport::Func(func) => func,
            _ => unreachable!(),
        };
        self.data.state.set_lowering(
            import.index,
            func.lowering(),
            self.component.lowering_ptr(import.index),
            self.component
                .signatures()
                .shared_signature(import.canonical_abi)
                .expect("found unregistered signature"),
        );

        // The `func` provided here must be retained within the `Store` itself
        // after instantiation. Otherwise it might be possible to drop the
        // `Arc<HostFunc>` and possibly result in a use-after-free. This comes
        // about because the `.lowering()` method returns a structure that
        // points to an interior pointer within the `func`. By saving the list
        // of host functions used we can ensure that the function lives long
        // enough for the whole duration of this instance.
        self.data.funcs.push(func.clone());
    }

    fn always_trap(&mut self, trap: &AlwaysTrap) {
        self.data.state.set_always_trap(
            trap.index,
            self.component.always_trap_ptr(trap.index),
            self.component
                .signatures()
                .shared_signature(trap.canonical_abi)
                .expect("found unregistered signature"),
        );
    }

    fn transcoder(&mut self, transcoder: &Transcoder) {
        self.data.state.set_transcoder(
            transcoder.index,
            self.component.transcoder_ptr(transcoder.index),
            self.component
                .signatures()
                .shared_signature(transcoder.signature)
                .expect("found unregistered signature"),
        );
    }

    fn extract_memory(&mut self, store: &mut StoreOpaque, memory: &ExtractMemory) {
        let mem = match self.data.lookup_export(store, &memory.export) {
            wasmtime_runtime::Export::Memory(m) => m,
            _ => unreachable!(),
        };
        self.data
            .state
            .set_runtime_memory(memory.index, mem.definition);
    }

    fn extract_realloc(&mut self, store: &mut StoreOpaque, realloc: &ExtractRealloc) {
        let anyfunc = match self.data.lookup_def(store, &realloc.def) {
            wasmtime_runtime::Export::Function(f) => f.anyfunc,
            _ => unreachable!(),
        };
        self.data.state.set_runtime_realloc(realloc.index, anyfunc);
    }

    fn extract_post_return(&mut self, store: &mut StoreOpaque, post_return: &ExtractPostReturn) {
        let anyfunc = match self.data.lookup_def(store, &post_return.def) {
            wasmtime_runtime::Export::Function(f) => f.anyfunc,
            _ => unreachable!(),
        };
        self.data
            .state
            .set_runtime_post_return(post_return.index, anyfunc);
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
                let (_, _, expected) = imports.next().unwrap();
                self.assert_type_matches(store, module, arg, expected);
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
        &mut self,
        store: &mut StoreOpaque,
        module: &Module,
        arg: &CoreDef,
        expected: EntityType,
    ) {
        let export = self.data.lookup_def(store, arg);

        // If this value is a core wasm function then the type check is inlined
        // here. This can otherwise fail `Extern::from_wasmtime_export` because
        // there's no guarantee that there exists a trampoline for `f` so this
        // can't fall through to the case below
        if let wasmtime_runtime::Export::Function(f) = &export {
            match expected {
                EntityType::Function(expected) => {
                    let actual = unsafe { f.anyfunc.as_ref().type_index };
                    assert_eq!(module.signatures().shared_signature(expected), Some(actual));
                    return;
                }
                _ => panic!("function not expected"),
            }
        }

        let val = unsafe { crate::Extern::from_wasmtime_export(export, store) };
        let ty = DefinitionType::from(store, &val);
        crate::types::matching::MatchCx {
            engine: store.engine(),
            signatures: module.signatures(),
            types: module.types(),
        }
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
    imports: PrimaryMap<RuntimeImportIndex, RuntimeImport>,
    _marker: marker::PhantomData<fn() -> T>,
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
            imports,
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
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
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
        let mut i = Instantiator::new(&self.component, store.0, &self.imports);
        i.run(&mut store)?;
        let data = Box::new(i.data);
        Ok(Instance(store.0.store_data_mut().insert(Some(data))))
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
            Export::Module(_) | Export::Instance(_) | Export::Type(_) => None,
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
            ._typed::<Params, Results>(self.store)
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
    }

    /// Same as [`Instance::get_module`]
    pub fn module(&mut self, name: &str) -> Option<&'a Module> {
        match self.exports.get(name)? {
            Export::Module(idx) => Some(&self.data.exported_modules[*idx]),
            _ => None,
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
        self.exports
            .iter()
            .filter_map(|(name, export)| match *export {
                Export::Module(idx) => Some((name.as_str(), &self.data.exported_modules[idx])),
                _ => None,
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
            Export::Instance(exports) => Some(ExportInstance {
                exports,
                instance: self.instance,
                data: self.data,
                store: self.store,
            }),
            _ => None,
        }
    }
}
