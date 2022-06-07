use crate::component::func::HostFunc;
use crate::component::{Component, ComponentParams, Func, Lift, Lower, TypedFunc};
use crate::instance::OwnedImports;
use crate::store::{StoreOpaque, Stored};
use crate::{AsContextMut, Module, StoreContext, StoreContextMut};
use anyhow::{anyhow, Context, Result};
use std::marker;
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, CoreDef, CoreExport, Export, ExportItem, Initializer, InstantiateModule,
    LowerImport, RuntimeImportIndex, RuntimeInstanceIndex, RuntimeMemoryIndex, RuntimeModuleIndex,
    RuntimeReallocIndex,
};
use wasmtime_environ::{EntityIndex, MemoryIndex, PrimaryMap};
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
    /// Looks up a function by name within this [`Instance`].
    ///
    /// The `store` specified must be the store that this instance lives within
    /// and `name` is the name of the function to lookup. If the function is
    /// found `Some` is returned otherwise `None` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_func(&self, mut store: impl AsContextMut, name: &str) -> Option<Func> {
        self._get_func(store.as_context_mut().0, name)
    }

    fn _get_func(&self, store: &mut StoreOpaque, name: &str) -> Option<Func> {
        // FIXME: this movement in ownership is unfortunate and feels like there
        // should be a better solution. The reason for this is that we need to
        // simultaneously look at lots of pieces of `InstanceData` while also
        // inserting into `store`, but `InstanceData` is stored within `store`.
        // By moving it out we appease the borrow-checker but take a runtime
        // hit.
        let data = store[self.0].take().unwrap();
        let result = data.get_func(store, self, name);
        store[self.0] = Some(data);
        return result;
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
        Params: ComponentParams + Lower,
        Results: Lift,
        S: AsContextMut,
    {
        let f = self
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(f.typed::<Params, Results, _>(store)
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
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
    pub fn modules<'a, T: 'a>(
        &'a self,
        store: impl Into<StoreContext<'a, T>>,
    ) -> impl Iterator<Item = (&'a str, &'a Module)> + 'a {
        let store = store.into();
        self._modules(store.0)
    }

    fn _modules<'a>(
        &'a self,
        store: &'a StoreOpaque,
    ) -> impl Iterator<Item = (&'a str, &'a Module)> + '_ {
        let data = store.store_data()[self.0].as_ref().unwrap();
        data.component
            .env_component()
            .exports
            .iter()
            .filter_map(|(name, export)| match *export {
                Export::Module(idx) => Some((name.as_str(), &data.exported_modules[idx])),
                _ => None,
            })
    }
}

impl InstanceData {
    fn get_func(&self, store: &mut StoreOpaque, instance: &Instance, name: &str) -> Option<Func> {
        match self.component.env_component().exports.get(name)? {
            Export::LiftedFunction { ty, func, options } => Some(Func::from_lifted_func(
                store, instance, self, *ty, func, options,
            )),
            Export::Module(_) => None,
        }
    }

    fn lookup_def(&self, store: &mut StoreOpaque, def: &CoreDef) -> wasmtime_runtime::Export {
        match def {
            CoreDef::Lowered(idx) => {
                wasmtime_runtime::Export::Function(wasmtime_runtime::ExportFunction {
                    anyfunc: self.state.lowering_anyfunc(*idx),
                })
            }
            CoreDef::Export(e) => self.lookup_export(store, e),
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
                Initializer::InstantiateModule(m) => {
                    let module;
                    let imports = match m {
                        // Since upvars are statically know we know that the
                        // `args` list is already in the right order.
                        InstantiateModule::Upvar(idx, args) => {
                            module = self.component.upvar(*idx);
                            self.build_imports(store.0, module, args.iter())
                        }

                        // With imports, unlike upvars, we need to do runtime
                        // lookups with strings to determine the order of the
                        // imports since it's whatever the actual module
                        // requires.
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
                    // already been performed. This maens that the unsafety due
                    // to imports having the wrong type should not happen here.
                    let i =
                        unsafe { crate::Instance::new_started(store, module, imports.as_ref())? };
                    self.data.instances.push(i);
                }

                Initializer::LowerImport(import) => self.lower_import(import),

                Initializer::ExtractMemory { index, export } => {
                    self.extract_memory(store.0, *index, export)
                }

                Initializer::ExtractRealloc { index, def } => {
                    self.extract_realloc(store.0, *index, def)
                }

                Initializer::SaveModuleUpvar(idx) => {
                    self.data
                        .exported_modules
                        .push(self.component.upvar(*idx).clone());
                }

                Initializer::SaveModuleImport(idx) => {
                    self.data.exported_modules.push(match &self.imports[*idx] {
                        RuntimeImport::Module(m) => m.clone(),
                        _ => unreachable!(),
                    });
                }
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
            self.component.trampoline_ptr(import.index),
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

    fn extract_memory(
        &mut self,
        store: &mut StoreOpaque,
        index: RuntimeMemoryIndex,
        export: &CoreExport<MemoryIndex>,
    ) {
        let memory = match self.data.lookup_export(store, export) {
            wasmtime_runtime::Export::Memory(m) => m,
            _ => unreachable!(),
        };
        self.data.state.set_runtime_memory(index, memory.definition);
    }

    fn extract_realloc(
        &mut self,
        store: &mut StoreOpaque,
        index: RuntimeReallocIndex,
        def: &CoreDef,
    ) {
        let anyfunc = match self.data.lookup_def(store, def) {
            wasmtime_runtime::Export::Function(f) => f.anyfunc,
            _ => unreachable!(),
        };
        self.data.state.set_runtime_realloc(index, anyfunc);
    }

    fn build_imports<'b>(
        &mut self,
        store: &mut StoreOpaque,
        module: &Module,
        args: impl Iterator<Item = &'b CoreDef>,
    ) -> &OwnedImports {
        self.core_imports.clear();
        self.core_imports.reserve(module);

        for arg in args {
            let export = self.data.lookup_def(store, arg);

            // The unsafety here should be ok since the `export` is loaded
            // directly from an instance which should only give us valid export
            // items.
            unsafe {
                self.core_imports.push_export(&export);
            }
        }

        &self.core_imports
    }
}

/// A "pre-instantiated" [`Instance`] which has all of its arguments already
/// supplied and is ready to instantiate.
///
/// This structure represents an efficient form of instantiation where import
/// type-checking and import lookup has all been resolved by the time that this
/// type is created. This type is primarily created through the
/// [`Linker::instance_pre`](crate::component::Linker::instance_pre) method.
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
    pub fn instantiate(&self, mut store: impl AsContextMut<Data = T>) -> Result<Instance> {
        let mut store = store.as_context_mut();
        let mut i = Instantiator::new(&self.component, store.0, &self.imports);
        i.run(&mut store)?;
        let data = Box::new(i.data);
        Ok(Instance(store.0.store_data_mut().insert(Some(data))))
    }
}
