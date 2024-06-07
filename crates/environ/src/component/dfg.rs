//! A dataflow-graph-like intermediate representation of a component
//!
//! This module contains `ComponentDfg` which is an intermediate step towards
//! becoming a full-fledged `Component`. The main purpose for the existence of
//! this representation of a component is to track dataflow between various
//! items within a component and support edits to them after the initial inlined
//! translation of a component.
//!
//! Currently fused adapters are represented with a core WebAssembly module
//! which gets "injected" into the final component as-if the component already
//! bundled it. In doing so the adapter modules need to be partitioned and
//! inserted into the final sequence of modules to instantiate. While this is
//! possible to do with a flat `GlobalInitializer` list it gets unwieldy really
//! quickly especially when other translation features are added.
//!
//! This module is largely a duplicate of the `component::info` module in this
//! crate. The hierarchy here uses `*Id` types instead of `*Index` types to
//! represent that they don't have any necessary implicit ordering. Additionally
//! nothing is kept in an ordered list and instead this is worked with in a
//! general dataflow fashion where dependencies are walked during processing.
//!
//! The `ComponentDfg::finish` method will convert the dataflow graph to a
//! linearized `GlobalInitializer` list which is intended to not be edited after
//! it's created.
//!
//! The `ComponentDfg` is created as part of the `component::inline` phase of
//! translation where the dataflow performed there allows identification of
//! fused adapters, what arguments make their way to core wasm modules, etc.

use crate::component::*;
use crate::prelude::*;
use crate::{EntityIndex, EntityRef, PrimaryMap, WasmValType};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Index;
use wasmtime_types::ModuleInternedTypeIndex;

#[derive(Default)]
#[allow(missing_docs)]
pub struct ComponentDfg {
    /// Same as `Component::import_types`
    pub import_types: PrimaryMap<ImportIndex, (String, TypeDef)>,

    /// Same as `Component::imports`
    pub imports: PrimaryMap<RuntimeImportIndex, (ImportIndex, Vec<String>)>,

    /// Same as `Component::exports`
    pub exports: IndexMap<String, Export>,

    /// All trampolines and their type signature which will need to get
    /// compiled by Cranelift.
    pub trampolines: Intern<TrampolineIndex, (ModuleInternedTypeIndex, Trampoline)>,

    /// Know reallocation functions which are used by `lowerings` (e.g. will be
    /// used by the host)
    pub reallocs: Intern<ReallocId, CoreDef>,

    /// Same as `reallocs`, but for post-return.
    pub post_returns: Intern<PostReturnId, CoreDef>,

    /// Same as `reallocs`, but for post-return.
    pub memories: Intern<MemoryId, CoreExport<MemoryIndex>>,

    /// Metadata about identified fused adapters.
    ///
    /// Note that this list is required to be populated in-order where the
    /// "left" adapters cannot depend on "right" adapters. Currently this falls
    /// out of the inlining pass of translation.
    pub adapters: Intern<AdapterId, Adapter>,

    /// Metadata about all known core wasm instances created.
    ///
    /// This is mostly an ordered list and is not deduplicated based on contents
    /// unlike the items above. Creation of an `Instance` is side-effectful and
    /// all instances here are always required to be created. These are
    /// considered "roots" in dataflow.
    pub instances: PrimaryMap<InstanceId, Instance>,

    /// Number of component instances that were created during the inlining
    /// phase (this is not edited after creation).
    pub num_runtime_component_instances: u32,

    /// Known adapter modules and how they are instantiated.
    ///
    /// This map is not filled in on the initial creation of a `ComponentDfg`.
    /// Instead these modules are filled in by the `inline::adapt` phase where
    /// adapter modules are identifed and filled in here.
    ///
    /// The payload here is the static module index representing the core wasm
    /// adapter module that was generated as well as the arguments to the
    /// instantiation of the adapter module.
    pub adapter_modules: PrimaryMap<AdapterModuleId, (StaticModuleIndex, Vec<CoreDef>)>,

    /// Metadata about where adapters can be found within their respective
    /// adapter modules.
    ///
    /// Like `adapter_modules` this is not filled on the initial creation of
    /// `ComponentDfg` but rather is created alongside `adapter_modules` during
    /// the `inline::adapt` phase of translation.
    ///
    /// The values here are the module that the adapter is present within along
    /// as the core wasm index of the export corresponding to the lowered
    /// version of the adapter.
    pub adapter_paritionings: PrimaryMap<AdapterId, (AdapterModuleId, EntityIndex)>,

    /// Defined resources in this component sorted by index with metadata about
    /// each resource.
    ///
    /// Note that each index here is a unique resource, and that may mean it was
    /// the same component instantiated twice for example.
    pub resources: PrimaryMap<DefinedResourceIndex, Resource>,

    /// Metadata about all imported resources into this component. This records
    /// both how many imported resources there are (the size of this map) along
    /// with what the corresponding runtime import is.
    pub imported_resources: PrimaryMap<ResourceIndex, RuntimeImportIndex>,

    /// The total number of resource tables that will be used by this component,
    /// currently the number of unique `TypeResourceTableIndex` allocations for
    /// this component.
    pub num_resource_tables: usize,

    /// An ordered list of side effects induced by instantiating this component.
    ///
    /// Currently all side effects are either instantiating core wasm modules or
    /// declaring a resource. These side effects affect the dataflow processing
    /// of this component by idnicating what order operations should be
    /// performed during instantiation.
    pub side_effects: Vec<SideEffect>,
}

/// Possible side effects that are possible with instantiating this component.
pub enum SideEffect {
    /// A core wasm instance was created.
    ///
    /// Instantiation is side-effectful due to the presence of constructs such
    /// as traps and the core wasm `start` function which may call component
    /// imports. Instantiation order from the original component must be done in
    /// the same order.
    Instance(InstanceId),

    /// A resource was declared in this component.
    ///
    /// This is a bit less side-effectful than instantiation but this serves as
    /// the order in which resources are initialized in a component with their
    /// destructors. Destructors are loaded from core wasm instances (or
    /// lowerings) which are produced by prior side-effectful operations.
    Resource(DefinedResourceIndex),
}

macro_rules! id {
    ($(pub struct $name:ident(u32);)*) => ($(
        #[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
        #[allow(missing_docs)]
        pub struct $name(u32);
        cranelift_entity::entity_impl!($name);
    )*)
}

id! {
    pub struct InstanceId(u32);
    pub struct MemoryId(u32);
    pub struct ReallocId(u32);
    pub struct AdapterId(u32);
    pub struct PostReturnId(u32);
    pub struct AdapterModuleId(u32);
}

/// Same as `info::InstantiateModule`
#[allow(missing_docs)]
pub enum Instance {
    Static(StaticModuleIndex, Box<[CoreDef]>),
    Import(
        RuntimeImportIndex,
        IndexMap<String, IndexMap<String, CoreDef>>,
    ),
}

/// Same as `info::Export`
#[allow(missing_docs)]
pub enum Export {
    LiftedFunction {
        ty: TypeFuncIndex,
        func: CoreDef,
        options: CanonicalOptions,
    },
    ModuleStatic(StaticModuleIndex),
    ModuleImport {
        ty: TypeModuleIndex,
        import: RuntimeImportIndex,
    },
    Instance {
        ty: Option<TypeComponentInstanceIndex>,
        exports: IndexMap<String, Export>,
    },
    Type(TypeDef),
}

/// Same as `info::CoreDef`, except has an extra `Adapter` variant.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum CoreDef {
    Export(CoreExport<EntityIndex>),
    InstanceFlags(RuntimeComponentInstanceIndex),
    Trampoline(TrampolineIndex),
    /// This is a special variant not present in `info::CoreDef` which
    /// represents that this definition refers to a fused adapter function. This
    /// adapter is fully processed after the initial translation and
    /// identificatino of adapters.
    ///
    /// During translation into `info::CoreDef` this variant is erased and
    /// replaced by `info::CoreDef::Export` since adapters are always
    /// represented as the exports of a core wasm instance.
    Adapter(AdapterId),
}

impl<T> From<CoreExport<T>> for CoreDef
where
    EntityIndex: From<T>,
{
    fn from(export: CoreExport<T>) -> CoreDef {
        CoreDef::Export(export.map_index(|i| i.into()))
    }
}

/// Same as `info::CoreExport`
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct CoreExport<T> {
    pub instance: InstanceId,
    pub item: ExportItem<T>,
}

impl<T> CoreExport<T> {
    #[allow(missing_docs)]
    pub fn map_index<U>(self, f: impl FnOnce(T) -> U) -> CoreExport<U> {
        CoreExport {
            instance: self.instance,
            item: match self.item {
                ExportItem::Index(i) => ExportItem::Index(f(i)),
                ExportItem::Name(s) => ExportItem::Name(s),
            },
        }
    }
}

/// Same as `info::Trampoline`
#[derive(Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Trampoline {
    LowerImport {
        import: RuntimeImportIndex,
        options: CanonicalOptions,
        lower_ty: TypeFuncIndex,
    },
    Transcoder {
        op: Transcode,
        from: MemoryId,
        from64: bool,
        to: MemoryId,
        to64: bool,
    },
    AlwaysTrap,
    ResourceNew(TypeResourceTableIndex),
    ResourceRep(TypeResourceTableIndex),
    ResourceDrop(TypeResourceTableIndex),
    ResourceTransferOwn,
    ResourceTransferBorrow,
    ResourceEnterCall,
    ResourceExitCall,
}

/// Same as `info::CanonicalOptions`
#[derive(Clone, Hash, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct CanonicalOptions {
    pub instance: RuntimeComponentInstanceIndex,
    pub string_encoding: StringEncoding,
    pub memory: Option<MemoryId>,
    pub realloc: Option<ReallocId>,
    pub post_return: Option<PostReturnId>,
}

/// Same as `info::Resource`
#[allow(missing_docs)]
pub struct Resource {
    pub rep: WasmValType,
    pub dtor: Option<CoreDef>,
    pub instance: RuntimeComponentInstanceIndex,
}

/// A helper structure to "intern" and deduplicate values of type `V` with an
/// identifying key `K`.
///
/// Note that this can also be used where `V` can't be intern'd to represent a
/// flat list of items.
pub struct Intern<K: EntityRef, V> {
    intern_map: HashMap<V, K>,
    key_map: PrimaryMap<K, V>,
}

impl<K, V> Intern<K, V>
where
    K: EntityRef,
{
    /// Inserts the `value` specified into this set, returning either a fresh
    /// key `K` if this value hasn't been seen before or otherwise returning the
    /// previous `K` used to represent value.
    ///
    /// Note that this should only be used for component model items where the
    /// creation of `value` is not side-effectful.
    pub fn push(&mut self, value: V) -> K
    where
        V: Hash + Eq + Clone,
    {
        *self
            .intern_map
            .entry(value.clone())
            .or_insert_with(|| self.key_map.push(value))
    }

    /// Returns an iterator of all the values contained within this set.
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        self.key_map.iter()
    }
}

impl<K: EntityRef, V> Index<K> for Intern<K, V> {
    type Output = V;
    fn index(&self, key: K) -> &V {
        &self.key_map[key]
    }
}

impl<K: EntityRef, V> Default for Intern<K, V> {
    fn default() -> Intern<K, V> {
        Intern {
            intern_map: HashMap::new(),
            key_map: PrimaryMap::new(),
        }
    }
}

impl ComponentDfg {
    /// Consumes the intermediate `ComponentDfg` to produce a final `Component`
    /// with a linear innitializer list.
    pub fn finish(self) -> ComponentTranslation {
        let mut linearize = LinearizeDfg {
            dfg: &self,
            initializers: Vec::new(),
            runtime_memories: Default::default(),
            runtime_post_return: Default::default(),
            runtime_reallocs: Default::default(),
            runtime_instances: Default::default(),
            num_lowerings: 0,
            trampolines: Default::default(),
            trampoline_defs: Default::default(),
            trampoline_map: Default::default(),
        };

        // Handle all side effects of this component in the order that they're
        // defined. This will, for example, process all instantiations necessary
        // of core wasm modules.
        for item in linearize.dfg.side_effects.iter() {
            linearize.side_effect(item);
        }

        // Next the exports of the instance are handled which will likely end up
        // creating some lowered imports, perhaps some saved modules, etc.
        let mut export_items = PrimaryMap::new();
        let exports = self
            .exports
            .iter()
            .map(|(name, export)| (name.clone(), linearize.export(export, &mut export_items)))
            .collect();

        // With all those pieces done the results of the dataflow-based
        // linearization are recorded into the `Component`. The number of
        // runtime values used for each index space is used from the `linearize`
        // result.
        ComponentTranslation {
            trampolines: linearize.trampoline_defs,
            component: Component {
                exports,
                export_items,
                initializers: linearize.initializers,
                trampolines: linearize.trampolines,
                num_lowerings: linearize.num_lowerings,

                num_runtime_memories: linearize.runtime_memories.len() as u32,
                num_runtime_post_returns: linearize.runtime_post_return.len() as u32,
                num_runtime_reallocs: linearize.runtime_reallocs.len() as u32,
                num_runtime_instances: linearize.runtime_instances.len() as u32,
                imports: self.imports,
                import_types: self.import_types,
                num_runtime_component_instances: self.num_runtime_component_instances,
                num_resource_tables: self.num_resource_tables,
                num_resources: (self.resources.len() + self.imported_resources.len()) as u32,
                imported_resources: self.imported_resources,
                defined_resource_instances: self
                    .resources
                    .iter()
                    .map(|(_, r)| r.instance)
                    .collect(),
            },
        }
    }

    /// Converts the provided defined index into a normal index, adding in the
    /// number of imported resources.
    pub fn resource_index(&self, defined: DefinedResourceIndex) -> ResourceIndex {
        ResourceIndex::from_u32(defined.as_u32() + (self.imported_resources.len() as u32))
    }
}

struct LinearizeDfg<'a> {
    dfg: &'a ComponentDfg,
    initializers: Vec<GlobalInitializer>,
    trampolines: PrimaryMap<TrampolineIndex, ModuleInternedTypeIndex>,
    trampoline_defs: PrimaryMap<TrampolineIndex, info::Trampoline>,
    trampoline_map: HashMap<TrampolineIndex, TrampolineIndex>,
    runtime_memories: HashMap<MemoryId, RuntimeMemoryIndex>,
    runtime_reallocs: HashMap<ReallocId, RuntimeReallocIndex>,
    runtime_post_return: HashMap<PostReturnId, RuntimePostReturnIndex>,
    runtime_instances: HashMap<RuntimeInstance, RuntimeInstanceIndex>,
    num_lowerings: u32,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
enum RuntimeInstance {
    Normal(InstanceId),
    Adapter(AdapterModuleId),
}

impl LinearizeDfg<'_> {
    fn side_effect(&mut self, effect: &SideEffect) {
        match effect {
            SideEffect::Instance(i) => {
                self.instantiate(*i, &self.dfg.instances[*i]);
            }
            SideEffect::Resource(i) => {
                self.resource(*i, &self.dfg.resources[*i]);
            }
        }
    }

    fn instantiate(&mut self, instance: InstanceId, args: &Instance) {
        log::trace!("creating instance {instance:?}");
        let instantiation = match args {
            Instance::Static(index, args) => InstantiateModule::Static(
                *index,
                args.iter().map(|def| self.core_def(def)).collect(),
            ),
            Instance::Import(index, args) => InstantiateModule::Import(
                *index,
                args.iter()
                    .map(|(module, values)| {
                        let values = values
                            .iter()
                            .map(|(name, def)| (name.clone(), self.core_def(def)))
                            .collect();
                        (module.clone(), values)
                    })
                    .collect(),
            ),
        };
        let index = RuntimeInstanceIndex::new(self.runtime_instances.len());
        self.initializers
            .push(GlobalInitializer::InstantiateModule(instantiation));
        let prev = self
            .runtime_instances
            .insert(RuntimeInstance::Normal(instance), index);
        assert!(prev.is_none());
    }

    fn resource(&mut self, index: DefinedResourceIndex, resource: &Resource) {
        let dtor = resource.dtor.as_ref().map(|dtor| self.core_def(dtor));
        self.initializers
            .push(GlobalInitializer::Resource(info::Resource {
                dtor,
                index,
                rep: resource.rep,
                instance: resource.instance,
            }));
    }

    fn export(
        &mut self,
        export: &Export,
        items: &mut PrimaryMap<ExportIndex, info::Export>,
    ) -> ExportIndex {
        let item = match export {
            Export::LiftedFunction { ty, func, options } => {
                let func = self.core_def(func);
                let options = self.options(options);
                info::Export::LiftedFunction {
                    ty: *ty,
                    func,
                    options,
                }
            }
            Export::ModuleStatic(i) => info::Export::ModuleStatic(*i),
            Export::ModuleImport { ty, import } => info::Export::ModuleImport {
                ty: *ty,
                import: *import,
            },
            Export::Instance { ty, exports } => info::Export::Instance {
                ty: *ty,
                exports: exports
                    .iter()
                    .map(|(name, export)| (name.clone(), self.export(export, items)))
                    .collect(),
            },
            Export::Type(def) => info::Export::Type(*def),
        };
        items.push(item)
    }

    fn options(&mut self, options: &CanonicalOptions) -> info::CanonicalOptions {
        let memory = options.memory.map(|mem| self.runtime_memory(mem));
        let realloc = options.realloc.map(|mem| self.runtime_realloc(mem));
        let post_return = options.post_return.map(|mem| self.runtime_post_return(mem));
        info::CanonicalOptions {
            instance: options.instance,
            string_encoding: options.string_encoding,
            memory,
            realloc,
            post_return,
        }
    }

    fn runtime_memory(&mut self, mem: MemoryId) -> RuntimeMemoryIndex {
        self.intern(
            mem,
            |me| &mut me.runtime_memories,
            |me, mem| me.core_export(&me.dfg.memories[mem]),
            |index, export| GlobalInitializer::ExtractMemory(ExtractMemory { index, export }),
        )
    }

    fn runtime_realloc(&mut self, realloc: ReallocId) -> RuntimeReallocIndex {
        self.intern(
            realloc,
            |me| &mut me.runtime_reallocs,
            |me, realloc| me.core_def(&me.dfg.reallocs[realloc]),
            |index, def| GlobalInitializer::ExtractRealloc(ExtractRealloc { index, def }),
        )
    }

    fn runtime_post_return(&mut self, post_return: PostReturnId) -> RuntimePostReturnIndex {
        self.intern(
            post_return,
            |me| &mut me.runtime_post_return,
            |me, post_return| me.core_def(&me.dfg.post_returns[post_return]),
            |index, def| GlobalInitializer::ExtractPostReturn(ExtractPostReturn { index, def }),
        )
    }

    fn core_def(&mut self, def: &CoreDef) -> info::CoreDef {
        match def {
            CoreDef::Export(e) => info::CoreDef::Export(self.core_export(e)),
            CoreDef::InstanceFlags(i) => info::CoreDef::InstanceFlags(*i),
            CoreDef::Adapter(id) => info::CoreDef::Export(self.adapter(*id)),
            CoreDef::Trampoline(index) => info::CoreDef::Trampoline(self.trampoline(*index)),
        }
    }

    fn trampoline(&mut self, index: TrampolineIndex) -> TrampolineIndex {
        if let Some(idx) = self.trampoline_map.get(&index) {
            return *idx;
        }
        let (signature, trampoline) = &self.dfg.trampolines[index];
        let trampoline = match trampoline {
            Trampoline::LowerImport {
                import,
                options,
                lower_ty,
            } => {
                let index = LoweredIndex::from_u32(self.num_lowerings);
                self.num_lowerings += 1;
                self.initializers.push(GlobalInitializer::LowerImport {
                    index,
                    import: *import,
                });
                info::Trampoline::LowerImport {
                    index,
                    options: self.options(options),
                    lower_ty: *lower_ty,
                }
            }
            Trampoline::Transcoder {
                op,
                from,
                from64,
                to,
                to64,
            } => info::Trampoline::Transcoder {
                op: *op,
                from: self.runtime_memory(*from),
                from64: *from64,
                to: self.runtime_memory(*to),
                to64: *to64,
            },
            Trampoline::AlwaysTrap => info::Trampoline::AlwaysTrap,
            Trampoline::ResourceNew(ty) => info::Trampoline::ResourceNew(*ty),
            Trampoline::ResourceDrop(ty) => info::Trampoline::ResourceDrop(*ty),
            Trampoline::ResourceRep(ty) => info::Trampoline::ResourceRep(*ty),
            Trampoline::ResourceTransferOwn => info::Trampoline::ResourceTransferOwn,
            Trampoline::ResourceTransferBorrow => info::Trampoline::ResourceTransferBorrow,
            Trampoline::ResourceEnterCall => info::Trampoline::ResourceEnterCall,
            Trampoline::ResourceExitCall => info::Trampoline::ResourceExitCall,
        };
        let i1 = self.trampolines.push(*signature);
        let i2 = self.trampoline_defs.push(trampoline);
        assert_eq!(i1, i2);
        self.trampoline_map.insert(index, i1);
        i1
    }

    fn core_export<T>(&mut self, export: &CoreExport<T>) -> info::CoreExport<T>
    where
        T: Clone,
    {
        let instance = export.instance;
        log::trace!("referencing export of {instance:?}");
        info::CoreExport {
            instance: self.runtime_instances[&RuntimeInstance::Normal(instance)],
            item: export.item.clone(),
        }
    }

    fn adapter(&mut self, adapter: AdapterId) -> info::CoreExport<EntityIndex> {
        let (adapter_module, entity_index) = self.dfg.adapter_paritionings[adapter];

        // Instantiates the adapter module if it hasn't already been
        // instantiated or otherwise returns the index that the module was
        // already instantiated at.
        let instance = self.adapter_module(adapter_module);

        // This adapter is always an export of the instance.
        info::CoreExport {
            instance,
            item: ExportItem::Index(entity_index),
        }
    }

    fn adapter_module(&mut self, adapter_module: AdapterModuleId) -> RuntimeInstanceIndex {
        self.intern(
            RuntimeInstance::Adapter(adapter_module),
            |me| &mut me.runtime_instances,
            |me, _| {
                log::debug!("instantiating {adapter_module:?}");
                let (module_index, args) = &me.dfg.adapter_modules[adapter_module];
                let args = args.iter().map(|arg| me.core_def(arg)).collect();
                let instantiate = InstantiateModule::Static(*module_index, args);
                GlobalInitializer::InstantiateModule(instantiate)
            },
            |_, init| init,
        )
    }

    /// Helper function to manage interning of results to avoid duplicate
    /// initializers being inserted into the final list.
    ///
    /// * `key` - the key being referenced which is used to deduplicate.
    /// * `map` - a closure to access the interning map on `Self`
    /// * `gen` - a closure to generate an intermediate value with `Self` from
    ///   `K`. This is only used if `key` hasn't previously been seen. This
    ///   closure can recursively intern other values possibly.
    /// * `init` - a closure to use the result of `gen` to create the final
    ///   initializer now that the index `V` of the runtime item is known.
    ///
    /// This is used by all the other interning methods above to lazily append
    /// initializers on-demand and avoid pushing more than one initializer at a
    /// time.
    fn intern<K, V, T>(
        &mut self,
        key: K,
        map: impl Fn(&mut Self) -> &mut HashMap<K, V>,
        gen: impl FnOnce(&mut Self, K) -> T,
        init: impl FnOnce(V, T) -> GlobalInitializer,
    ) -> V
    where
        K: Hash + Eq + Copy,
        V: EntityRef,
    {
        if let Some(val) = map(self).get(&key) {
            return val.clone();
        }
        let tmp = gen(self, key);
        let index = V::new(map(self).len());
        self.initializers.push(init(index, tmp));
        let prev = map(self).insert(key, index);
        assert!(prev.is_none());
        index
    }
}
