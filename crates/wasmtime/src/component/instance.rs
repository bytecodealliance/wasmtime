use crate::component::{Component, ComponentParams, ComponentValue, Func, TypedFunc};
use crate::instance::OwnedImports;
use crate::store::{StoreOpaque, Stored};
use crate::{AsContextMut, Module, StoreContextMut};
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, CoreDef, CoreExport, Export, ExportItem, Initializer, ModuleToInstantiate,
    RuntimeInstanceIndex, RuntimeMemoryIndex, RuntimeReallocIndex,
};
use wasmtime_environ::{EntityIndex, PrimaryMap};

/// An instantiated component.
///
/// This is similar to [`crate::Instance`] except that it represents an
/// instantiated component instead of an instantiated module. Otherwise though
/// the two behave similarly.
//
// FIXME: need to write more docs here.
#[derive(Copy, Clone)]
pub struct Instance(Stored<Option<Box<InstanceData>>>);

pub(crate) struct InstanceData {
    instances: PrimaryMap<RuntimeInstanceIndex, crate::Instance>,
    // FIXME: shouldn't store the entire component here which keeps upvars
    // alive and things like that, instead only the bare minimum necessary
    // should be kept alive here (mostly just `wasmtime_environ::Component`.
    component: Component,

    // TODO: move these to `VMComponentContext`
    memories: PrimaryMap<RuntimeMemoryIndex, wasmtime_runtime::ExportMemory>,
    reallocs: PrimaryMap<RuntimeReallocIndex, wasmtime_runtime::ExportFunction>,
}

impl Instance {
    /// Instantiates the `component` provided within the given `store`.
    ///
    /// Does not support components which have imports at this time.
    //
    // FIXME: need to write more docs here.
    pub fn new(mut store: impl AsContextMut, component: &Component) -> Result<Instance> {
        let mut store = store.as_context_mut();

        let mut instantiator = Instantiator::new(component);
        instantiator.run(&mut store)?;

        let data = Box::new(instantiator.data);
        Ok(Instance(store.0.store_data_mut().insert(Some(data))))
    }

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
        let result = data.get_func(store, name);
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
        Params: ComponentParams,
        Results: ComponentValue,
        S: AsContextMut,
    {
        let f = self
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(f.typed::<Params, Results, _>(store)
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
    }
}

impl InstanceData {
    fn get_func(&self, store: &mut StoreOpaque, name: &str) -> Option<Func> {
        match self.component.env_component().exports.get(name)? {
            Export::LiftedFunction { ty, func, options } => {
                Some(Func::from_lifted_func(store, self, *ty, func, options))
            }
        }
    }

    fn lookup_def(&self, store: &mut StoreOpaque, item: &CoreDef) -> wasmtime_runtime::Export {
        match item {
            CoreDef::Lowered(_) => unimplemented!(),
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

    pub fn component_types(&self) -> &Arc<ComponentTypes> {
        self.component.types()
    }

    pub fn runtime_memory(&self, memory: RuntimeMemoryIndex) -> wasmtime_runtime::ExportMemory {
        self.memories[memory].clone()
    }

    pub fn runtime_realloc(
        &self,
        realloc: RuntimeReallocIndex,
    ) -> wasmtime_runtime::ExportFunction {
        self.reallocs[realloc].clone()
    }
}

struct Instantiator<'a> {
    component: &'a Component,
    data: InstanceData,
    imports: OwnedImports,
}

impl<'a> Instantiator<'a> {
    fn new(component: &'a Component) -> Instantiator<'a> {
        let env_component = component.env_component();
        if env_component.imports.len() > 0 {
            unimplemented!("component imports");
        }
        Instantiator {
            component,
            imports: OwnedImports::empty(),
            data: InstanceData {
                instances: PrimaryMap::with_capacity(env_component.num_runtime_instances as usize),
                component: component.clone(),
                memories: Default::default(),
                reallocs: Default::default(),
            },
        }
    }

    fn run<T>(&mut self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        let env_component = self.component.env_component();
        for initializer in env_component.initializers.iter() {
            match initializer {
                Initializer::InstantiateModule {
                    instance,
                    module,
                    args,
                } => {
                    let module = match module {
                        ModuleToInstantiate::Upvar(module) => self.component.upvar(*module),
                        ModuleToInstantiate::Import(idx) => {
                            drop(idx);
                            unimplemented!("component module imports");
                        }
                    };

                    // Note that the unsafety here should be ok because the
                    // validity of the component means that type-checks have
                    // already been performed. This maens that the unsafety due
                    // to imports having the wrong type should not happen here.
                    let imports = self.build_imports(store.0, module, args);
                    let i =
                        unsafe { crate::Instance::new_started(store, module, imports.as_ref())? };
                    let idx = self.data.instances.push(i);
                    assert_eq!(idx, *instance);
                }
                Initializer::LowerImport(_) => unimplemented!(),

                Initializer::ExtractMemory { index, export } => {
                    let memory = match self.data.lookup_export(store.0, export) {
                        wasmtime_runtime::Export::Memory(m) => m,
                        _ => unreachable!(),
                    };
                    assert_eq!(*index, self.data.memories.push(memory));
                }

                Initializer::ExtractRealloc { index, def } => {
                    let func = match self.data.lookup_def(store.0, def) {
                        wasmtime_runtime::Export::Function(f) => f,
                        _ => unreachable!(),
                    };
                    assert_eq!(*index, self.data.reallocs.push(func));
                }
            }
        }
        Ok(())
    }

    fn build_imports(
        &mut self,
        store: &mut StoreOpaque,
        module: &Module,
        args: &[CoreDef],
    ) -> &OwnedImports {
        self.imports.clear();
        self.imports.reserve(module);

        for arg in args {
            let export = self.data.lookup_def(store, arg);

            // The unsafety here should be ok since the `export` is loaded
            // directly from an instance which should only give us valid export
            // items.
            unsafe {
                self.imports.push_export(&export);
            }
        }

        &self.imports
    }
}
