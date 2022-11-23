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
use crate::{EntityIndex, EntityRef, PrimaryMap, SignatureIndex};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Index;

#[derive(Default)]
#[allow(missing_docs)]
pub struct ComponentDfg {
    /// Same as `Component::import_types`
    pub import_types: PrimaryMap<ImportIndex, (String, TypeDef)>,

    /// Same as `Component::imports`
    pub imports: PrimaryMap<RuntimeImportIndex, (ImportIndex, Vec<String>)>,

    /// Same as `Component::exports`
    pub exports: IndexMap<String, Export>,

    /// All known lowered host functions along with the configuration for each
    /// lowering.
    pub lowerings: Intern<LowerImportId, LowerImport>,

    /// All known "always trapping" trampolines and the function signature they
    /// have.
    pub always_trap: Intern<AlwaysTrapId, SignatureIndex>,

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

    /// Metadata about string transcoders needed by adapter modules.
    pub transcoders: Intern<TranscoderId, Transcoder>,

    /// Metadata about all known core wasm instances created.
    ///
    /// This is mostly an ordered list and is not deduplicated based on contents
    /// unlike the items above. Creation of an `Instance` is side-effectful and
    /// all instances here are always required to be created. These are
    /// considered "roots" in dataflow.
    pub instances: Intern<InstanceId, Instance>,

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
    pub struct LowerImportId(u32);
    pub struct MemoryId(u32);
    pub struct ReallocId(u32);
    pub struct AdapterId(u32);
    pub struct PostReturnId(u32);
    pub struct AlwaysTrapId(u32);
    pub struct AdapterModuleId(u32);
    pub struct TranscoderId(u32);
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
    ModuleImport(RuntimeImportIndex),
    Instance(IndexMap<String, Export>),
    Type(TypeDef),
}

/// Same as `info::CoreDef`, except has an extra `Adapter` variant.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum CoreDef {
    Export(CoreExport<EntityIndex>),
    Lowered(LowerImportId),
    AlwaysTrap(AlwaysTrapId),
    InstanceFlags(RuntimeComponentInstanceIndex),
    Transcoder(TranscoderId),

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

/// Same as `info::LowerImport`
#[derive(Hash, Eq, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct LowerImport {
    pub import: RuntimeImportIndex,
    pub canonical_abi: SignatureIndex,
    pub options: CanonicalOptions,
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

/// Same as `info::Transcoder`
#[derive(Clone, Hash, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct Transcoder {
    pub op: Transcode,
    pub from: MemoryId,
    pub from64: bool,
    pub to: MemoryId,
    pub to64: bool,
    pub signature: SignatureIndex,
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
    /// Pushes a new `value` into this list without interning, assigning a new
    /// unique key `K` to the value.
    pub fn push(&mut self, value: V) -> K {
        self.key_map.push(value)
    }

    /// Inserts the `value` specified into this set, returning either a fresh
    /// key `K` if this value hasn't been seen before or otherwise returning the
    /// previous `K` used to represent value.
    ///
    /// Note that this should only be used for component model items where the
    /// creation of `value` is not side-effectful.
    pub fn push_uniq(&mut self, value: V) -> K
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
    pub fn finish(self) -> Component {
        let mut linearize = LinearizeDfg {
            dfg: &self,
            initializers: Vec::new(),
            num_runtime_modules: 0,
            runtime_memories: Default::default(),
            runtime_post_return: Default::default(),
            runtime_reallocs: Default::default(),
            runtime_instances: Default::default(),
            runtime_always_trap: Default::default(),
            runtime_lowerings: Default::default(),
            runtime_transcoders: Default::default(),
        };

        // First the instances are all processed for instantiation. This will,
        // recursively, handle any arguments necessary for each instance such as
        // instantiation of adapter modules.
        for (id, instance) in linearize.dfg.instances.key_map.iter() {
            linearize.instantiate(id, instance);
        }

        // Second the exports of the instance are handled which will likely end
        // up creating some lowered imports, perhaps some saved modules, etc.
        let exports = self
            .exports
            .iter()
            .map(|(name, export)| (name.clone(), linearize.export(export)))
            .collect();

        // With all those pieces done the results of the dataflow-based
        // linearization are recorded into the `Component`. The number of
        // runtime values used for each index space is used from the `linearize`
        // result.
        Component {
            exports,
            initializers: linearize.initializers,

            num_runtime_modules: linearize.num_runtime_modules,
            num_runtime_memories: linearize.runtime_memories.len() as u32,
            num_runtime_post_returns: linearize.runtime_post_return.len() as u32,
            num_runtime_reallocs: linearize.runtime_reallocs.len() as u32,
            num_runtime_instances: linearize.runtime_instances.len() as u32,
            num_always_trap: linearize.runtime_always_trap.len() as u32,
            num_lowerings: linearize.runtime_lowerings.len() as u32,
            num_transcoders: linearize.runtime_transcoders.len() as u32,

            imports: self.imports,
            import_types: self.import_types,
            num_runtime_component_instances: self.num_runtime_component_instances,
        }
    }
}

struct LinearizeDfg<'a> {
    dfg: &'a ComponentDfg,
    initializers: Vec<GlobalInitializer>,
    num_runtime_modules: u32,
    runtime_memories: HashMap<MemoryId, RuntimeMemoryIndex>,
    runtime_reallocs: HashMap<ReallocId, RuntimeReallocIndex>,
    runtime_post_return: HashMap<PostReturnId, RuntimePostReturnIndex>,
    runtime_instances: HashMap<RuntimeInstance, RuntimeInstanceIndex>,
    runtime_always_trap: HashMap<AlwaysTrapId, RuntimeAlwaysTrapIndex>,
    runtime_lowerings: HashMap<LowerImportId, LoweredIndex>,
    runtime_transcoders: HashMap<TranscoderId, RuntimeTranscoderIndex>,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
enum RuntimeInstance {
    Normal(InstanceId),
    Adapter(AdapterModuleId),
}

impl LinearizeDfg<'_> {
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

    fn export(&mut self, export: &Export) -> info::Export {
        match export {
            Export::LiftedFunction { ty, func, options } => {
                let func = self.core_def(func);
                let options = self.options(options);
                info::Export::LiftedFunction {
                    ty: *ty,
                    func,
                    options,
                }
            }
            Export::ModuleStatic(i) => {
                let index = RuntimeModuleIndex::from_u32(self.num_runtime_modules);
                self.num_runtime_modules += 1;
                self.initializers
                    .push(GlobalInitializer::SaveStaticModule(*i));
                info::Export::Module(index)
            }
            Export::ModuleImport(i) => {
                let index = RuntimeModuleIndex::from_u32(self.num_runtime_modules);
                self.num_runtime_modules += 1;
                self.initializers
                    .push(GlobalInitializer::SaveModuleImport(*i));
                info::Export::Module(index)
            }
            Export::Instance(map) => info::Export::Instance(
                map.iter()
                    .map(|(name, export)| (name.clone(), self.export(export)))
                    .collect(),
            ),
            Export::Type(def) => info::Export::Type(*def),
        }
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
            CoreDef::AlwaysTrap(id) => info::CoreDef::AlwaysTrap(self.runtime_always_trap(*id)),
            CoreDef::Lowered(id) => info::CoreDef::Lowered(self.runtime_lowering(*id)),
            CoreDef::InstanceFlags(i) => info::CoreDef::InstanceFlags(*i),
            CoreDef::Adapter(id) => info::CoreDef::Export(self.adapter(*id)),
            CoreDef::Transcoder(id) => info::CoreDef::Transcoder(self.runtime_transcoder(*id)),
        }
    }

    fn runtime_always_trap(&mut self, id: AlwaysTrapId) -> RuntimeAlwaysTrapIndex {
        self.intern(
            id,
            |me| &mut me.runtime_always_trap,
            |me, id| me.dfg.always_trap[id],
            |index, canonical_abi| {
                GlobalInitializer::AlwaysTrap(AlwaysTrap {
                    index,
                    canonical_abi,
                })
            },
        )
    }

    fn runtime_lowering(&mut self, id: LowerImportId) -> LoweredIndex {
        self.intern(
            id,
            |me| &mut me.runtime_lowerings,
            |me, id| {
                let info = &me.dfg.lowerings[id];
                let options = me.options(&info.options);
                (info.import, info.canonical_abi, options)
            },
            |index, (import, canonical_abi, options)| {
                GlobalInitializer::LowerImport(info::LowerImport {
                    index,
                    import,
                    canonical_abi,
                    options,
                })
            },
        )
    }

    fn runtime_transcoder(&mut self, id: TranscoderId) -> RuntimeTranscoderIndex {
        self.intern(
            id,
            |me| &mut me.runtime_transcoders,
            |me, id| {
                let info = &me.dfg.transcoders[id];
                (
                    info.op,
                    me.runtime_memory(info.from),
                    info.from64,
                    me.runtime_memory(info.to),
                    info.to64,
                    info.signature,
                )
            },
            |index, (op, from, from64, to, to64, signature)| {
                GlobalInitializer::Transcoder(info::Transcoder {
                    index,
                    op,
                    from,
                    from64,
                    to,
                    to64,
                    signature,
                })
            },
        )
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
