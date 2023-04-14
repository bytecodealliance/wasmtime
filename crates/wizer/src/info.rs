use wasm_encoder::SectionId;

pub mod types_interner;

use std::collections::BTreeMap;
use std::convert::TryFrom;
use types_interner::{EntityType, InstanceType, Type, TypeId, TypesInterner};

/// A collection of info about modules within a module linking bundle.
pub(crate) struct ModuleContext<'a> {
    arena: Vec<ModuleInfo<'a>>,
    types: TypesInterner<'a>,
}

impl<'a> ModuleContext<'a> {
    /// Construct a new `ModuleContext`, pre-populated with an empty root
    /// module.
    pub fn new() -> Self {
        Self {
            arena: vec![ModuleInfo::Defined(DefinedModuleInfo::default())],
            types: TypesInterner::default(),
        }
    }

    /// Get the root module.
    pub fn root(&self) -> Module {
        Module { id: 0 }
    }

    /// Get the interned types set for this module context.
    pub fn types(&self) -> &TypesInterner<'a> {
        &self.types
    }

    /// Does this context represent a single Wasm module that doesn't use module
    /// linking, or does it represent a bundle of one or more Wasm modules that
    /// use module linking?
    pub fn uses_module_linking(&self) -> bool {
        self.arena.len() > 1
            || self.root().initial_sections(self).any(|s| {
                s.id == SectionId::Alias.into()
                    || s.id == SectionId::Module.into()
                    || s.id == SectionId::Instance.into()
            })
    }

    /// Get a shared reference to the `DefinedModuleInfo` for this module,
    /// following through aliases.
    fn defined(&self, module: Module) -> &DefinedModuleInfo<'a> {
        let mut id = module.id;
        loop {
            match &self.arena[id] {
                ModuleInfo::Aliased(AliasedModuleInfo { alias_of, .. }) => {
                    id = *alias_of;
                }
                ModuleInfo::Defined(d) => return d,
            }
        }
    }

    /// Get an exclusive reference to the `DefinedModuleInfo` for this module.
    ///
    /// Does not resolve through aliases, because you shouldn't ever mutate
    /// aliased modules.
    fn defined_mut(&mut self, module: Module) -> &mut DefinedModuleInfo<'a> {
        match &mut self.arena[module.id] {
            ModuleInfo::Aliased(_) => panic!("not a defined module"),
            ModuleInfo::Defined(d) => d,
        }
    }
}

enum ModuleInfo<'a> {
    Aliased(AliasedModuleInfo),
    Defined(DefinedModuleInfo<'a>),
}

struct AliasedModuleInfo {
    /// The id of the other module that this is an alias of.
    pub alias_of: usize,
}

/// Info that we keep track of on a per module-within-a-module-linking-bundle
/// basis.
///
/// These are created during during our `parse` pass and then used throughout
/// our later passes.
#[derive(Default)]
struct DefinedModuleInfo<'a> {
    /// The raw sections from the original Wasm input.
    raw_sections: Vec<wasm_encoder::RawSection<'a>>,

    /// This vector has `n` entries when the module has `n` import sections. The
    /// `i`th entry is a count of how many instance imports are in the `i`th
    /// import section.
    instance_import_counts: Vec<u32>,

    /// Types available in this module.
    ///
    /// We keep track of these for determining how many things we need to
    /// re-export for new instantiations and for inner module's aliases.
    types: Vec<TypeId>,

    /// Imports made by this module.
    imports: Vec<wasmparser::Import<'a>>,

    /// Aliases that this module defines.
    aliases: Vec<wasmparser::Alias<'a>>,

    /// Directly nested inner modules of this module.
    ///
    /// These entries are populated as we finish instrumenting the inner
    /// modules.
    modules: Vec<Module>,

    /// A map from instance indices to each instance's type for all defined,
    /// imported, and aliased instances.
    instances: Vec<TypeId>,

    /// A map from indices of defined instantiations (as opposed to imported or
    /// aliased instantiations) to the id of the module that was instantiated
    /// and the import arguments.
    instantiations: BTreeMap<u32, (Module, Vec<wasmparser::InstanceArg<'a>>)>,

    /// A map from global indices to each global's type for all defined,
    /// imported, and aliased globals.
    globals: Vec<wasmparser::GlobalType>,

    /// The index within the global index space where defined globals (as
    /// opposed to imported or aliased) begin.
    ///
    /// If this is `None`, then there are no locally defined globals.
    defined_globals_index: Option<u32>,

    /// This module's exports.
    ///
    /// This is used later on, in the rewrite phase, when we are inserting state
    /// instance imports.
    ///
    /// Note that this does *not* include the `__wizer_thing_N` exports that
    /// this instrumentation pass adds.
    exports: Vec<wasmparser::Export<'a>>,

    /// Maps from function index to the function's type index for all functions
    /// defined, imported, and aliased in this module.
    functions: Vec<TypeId>,

    /// Maps from table index to the table's type for all tables defined,
    /// imported, and aliased in this module.
    tables: Vec<wasmparser::TableType>,

    /// Maps from memory index to the memory's type for all memories defined,
    /// imported, and aliased in this module.
    memories: Vec<wasmparser::MemoryType>,

    /// The index within the memory index space where defined memories (as
    /// opposed to imported or aliased) begin.
    ///
    /// If this is `None`, then there are no locally defined memories.
    defined_memories_index: Option<u32>,
}

/// A module inside a module linking bundle.
///
/// This is a small, copy-able type that essentially just indexes into a
/// `ModuleContext`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Module {
    /// This module's id, aka its pre-order traversal index, aka its index in
    /// `Modules::arena`.
    id: usize,
}

impl Module {
    /// Construct a new, defined module.
    pub fn new_defined(cx: &mut ModuleContext) -> Self {
        let id = cx.arena.len();
        cx.arena.push(ModuleInfo::Defined(Default::default()));
        Module { id }
    }

    /// Construct a new module that is an alias of the given module.
    pub fn new_aliased(cx: &mut ModuleContext, alias_of: Module) -> Self {
        let id = cx.arena.len();
        cx.arena.push(ModuleInfo::Aliased(AliasedModuleInfo {
            alias_of: alias_of.id,
        }));
        Module { id }
    }

    /// Get the pre-order traversal index of this module in its associated
    /// module linking bundle.
    pub fn pre_order_index(self) -> u32 {
        u32::try_from(self.id).unwrap()
    }

    /// Get the defined module that this module is an alias of, if any.
    ///
    /// This will see through all aliases.
    pub fn get_aliased(self, cx: &ModuleContext<'_>) -> Option<Module> {
        if matches!(cx.arena[self.id], ModuleInfo::Defined(_)) {
            return None;
        }

        let mut id = self.id;
        loop {
            match &cx.arena[id] {
                ModuleInfo::Aliased(AliasedModuleInfo { alias_of, .. }) => {
                    id = *alias_of;
                }
                ModuleInfo::Defined(_) => return Some(Module { id }),
            }
        }
    }

    /// Translate the given `wasmparser` entity type into its interned
    /// representation using this module's types space.
    pub fn entity_type(
        self,
        cx: &ModuleContext<'_>,
        ty: wasmparser::ImportSectionEntryType,
    ) -> EntityType {
        cx.types().entity_type(ty, &cx.defined(self).types)
    }

    /// Add a new raw section to this module info during parsing.
    pub fn add_raw_section<'a>(
        self,
        cx: &mut ModuleContext<'a>,
        id: SectionId,
        range: wasmparser::Range,
        full_wasm: &'a [u8],
    ) {
        cx.defined_mut(self)
            .raw_sections
            .push(wasm_encoder::RawSection {
                id: id as u8,
                data: &full_wasm[range.start..range.end],
            })
    }

    /// Push a new type into this module's types space.
    pub fn push_type<'a>(self, cx: &mut ModuleContext<'a>, ty: wasmparser::TypeDef<'a>) {
        let types_space = match &cx.arena[self.id] {
            ModuleInfo::Aliased(_) => panic!("not a defined module"),
            ModuleInfo::Defined(d) => &d.types,
        };
        let ty = cx.types.insert_wasmparser(ty, types_space);
        cx.defined_mut(self).types.push(ty);
    }

    /// Push an aliased type into this module's types space.
    pub fn push_aliased_type(self, cx: &mut ModuleContext<'_>, ty: TypeId) {
        cx.defined_mut(self).types.push(ty);
    }

    /// Push a new module onto this module's list of nested child modules.
    pub fn push_child_module(self, cx: &mut ModuleContext<'_>, child: Module) {
        cx.defined_mut(self).modules.push(child);
    }

    /// Push a new, aliased instance into this module's instance index space.
    pub fn push_aliased_instance<'a>(self, cx: &mut ModuleContext<'a>, instance_type: TypeId) {
        assert!(cx.types.get(instance_type).is_instance());
        cx.defined_mut(self).instances.push(instance_type);
    }

    /// Push a new, imported instance into this module's instance index space.
    pub fn push_imported_instance<'a>(self, cx: &mut ModuleContext<'a>, instance_type: TypeId) {
        assert!(cx.types.get(instance_type).is_instance());
        cx.defined_mut(self).instances.push(instance_type);
    }

    /// Push a new, imported instance into this module's instance index space.
    pub fn push_defined_instance<'a>(
        self,
        cx: &mut ModuleContext<'a>,
        instance_type: TypeId,
        module: Module,
        args: Vec<wasmparser::InstanceArg<'a>>,
    ) {
        assert!(cx.types.get(instance_type).is_instance());
        let info = cx.defined_mut(self);
        let index = u32::try_from(info.instances.len()).unwrap();
        info.instances.push(instance_type);
        info.instantiations.insert(index, (module, args));
    }

    /// Push a new imported memory into this module's memory index space.
    pub fn push_imported_memory(self, cx: &mut ModuleContext, memory_type: wasmparser::MemoryType) {
        let info = cx.defined_mut(self);
        assert!(info.defined_memories_index.is_none());
        info.memories.push(memory_type);
    }

    /// Push a new defined memory into this module's memory index space.
    pub fn push_defined_memory(self, cx: &mut ModuleContext, memory_type: wasmparser::MemoryType) {
        let info = cx.defined_mut(self);
        if info.defined_memories_index.is_none() {
            info.defined_memories_index = Some(u32::try_from(info.memories.len()).unwrap());
        }
        info.memories.push(memory_type);
    }

    /// Push a new imported global into this module's global index space.
    pub fn push_imported_global(self, cx: &mut ModuleContext, global_type: wasmparser::GlobalType) {
        let info = cx.defined_mut(self);
        assert!(info.defined_globals_index.is_none());
        info.globals.push(global_type);
    }

    /// Push a new defined global into this module's global index space.
    pub fn push_defined_global(self, cx: &mut ModuleContext, global_type: wasmparser::GlobalType) {
        let info = cx.defined_mut(self);
        if info.defined_globals_index.is_none() {
            info.defined_globals_index = Some(u32::try_from(info.globals.len()).unwrap());
        }
        info.globals.push(global_type);
    }

    /// Push a new function into this module's function index space.
    pub fn push_function(self, cx: &mut ModuleContext, func_type: TypeId) {
        assert!(cx.types.get(func_type).is_func());
        cx.defined_mut(self).functions.push(func_type);
    }

    /// Push a new table into this module's table index space.
    pub fn push_table(self, cx: &mut ModuleContext, table_type: wasmparser::TableType) {
        cx.defined_mut(self).tables.push(table_type);
    }

    /// Push a new import into this module.
    pub fn push_import<'a>(self, cx: &mut ModuleContext<'a>, import: wasmparser::Import<'a>) {
        cx.defined_mut(self).imports.push(import);

        // Add the import to the appropriate index space for our current module.
        match import.ty {
            wasmparser::ImportSectionEntryType::Memory(ty) => {
                self.push_imported_memory(cx, ty);
            }
            wasmparser::ImportSectionEntryType::Global(ty) => {
                self.push_imported_global(cx, ty);
            }
            wasmparser::ImportSectionEntryType::Instance(ty_idx) => {
                let ty = self.instance_type_at(cx, ty_idx).clone();
                self.push_imported_instance(cx, ty);
            }
            wasmparser::ImportSectionEntryType::Function(ty_idx) => {
                let ty = self.type_id_at(cx, ty_idx);
                self.push_function(cx, ty);
            }
            wasmparser::ImportSectionEntryType::Table(ty) => {
                self.push_table(cx, ty);
            }
            wasmparser::ImportSectionEntryType::Module(_) => {
                unreachable!("we disallow module imports; checked in validation")
            }
            wasmparser::ImportSectionEntryType::Tag(_) => {
                unreachable!("exceptions are unsupported; checked in validation")
            }
        }
    }

    /// Push a count of how many instance imports an import section had.
    pub fn push_instance_import_count(self, cx: &mut ModuleContext<'_>, count: u32) {
        cx.defined_mut(self).instance_import_counts.push(count);
    }

    /// Push an instance implicitly created by a two-level import into this
    /// module.
    pub fn push_implicit_instance<'a>(
        self,
        cx: &mut ModuleContext<'a>,
        instance_type: InstanceType<'a>,
    ) {
        let ty = cx.types.insert(Type::Instance(instance_type));
        cx.defined_mut(self).instances.push(ty);
    }

    /// Push an alias into this module.
    pub fn push_alias<'a>(self, cx: &mut ModuleContext<'a>, alias: wasmparser::Alias<'a>) {
        cx.defined_mut(self).aliases.push(alias);
    }

    /// Push an export into this module.
    pub fn push_export<'a>(self, cx: &mut ModuleContext<'a>, export: wasmparser::Export<'a>) {
        cx.defined_mut(self).exports.push(export);
    }

    /// Is this the root of the module linking bundle?
    pub fn is_root(self) -> bool {
        self.id == 0
    }

    /// Define an instance type for this module's exports.
    ///
    /// Returns the index of the type and updates the total count of types in
    /// `num_types`.
    pub fn define_instance_type(self, cx: &mut ModuleContext<'_>) -> TypeId {
        // Inline `cx.defined(self)` to avoid borrowck errors.
        let info = {
            let mut id = self.id;
            loop {
                match &cx.arena[id] {
                    ModuleInfo::Aliased(AliasedModuleInfo { alias_of, .. }) => {
                        id = *alias_of;
                    }
                    ModuleInfo::Defined(d) => break d,
                }
            }
        };

        cx.types.insert(Type::Instance(InstanceType {
            exports: info
                .exports
                .iter()
                .map(|e| {
                    let name = e.field.into();
                    let index = usize::try_from(e.index).unwrap();
                    let entity = match e.kind {
                        wasmparser::ExternalKind::Function => {
                            let func_ty = info.functions[index];
                            EntityType::Function(func_ty)
                        }
                        wasmparser::ExternalKind::Table => {
                            let ty = info.tables[index];
                            EntityType::Table(ty)
                        }
                        wasmparser::ExternalKind::Memory => {
                            let ty = info.memories[index];
                            EntityType::Memory(ty)
                        }
                        wasmparser::ExternalKind::Global => {
                            let ty = info.globals[index];
                            EntityType::Global(ty)
                        }
                        wasmparser::ExternalKind::Instance => {
                            EntityType::Instance(info.instances[index])
                        }
                        wasmparser::ExternalKind::Module
                        | wasmparser::ExternalKind::Type
                        | wasmparser::ExternalKind::Tag => unreachable!(),
                    };
                    (name, entity)
                })
                .collect(),
        }))
    }

    /// Define an instance type for this module's state.
    pub fn define_state_instance_type(self, cx: &mut ModuleContext<'_>) -> TypeId {
        // Define instance types for each of the instances that we instantiate
        // locally so that we can refer to these types in the state instance's
        // type.
        let instantiated_modules: Vec<_> =
            self.instantiations(cx).values().map(|(m, _)| *m).collect();
        let instance_types = instantiated_modules
            .into_iter()
            .map(|m| m.define_instance_type(cx))
            .collect::<Vec<_>>();

        // Define the state instance type.
        cx.types.insert(Type::Instance(InstanceType {
            exports: self
                .defined_globals(cx)
                .enumerate()
                .map(|(i, (_, g))| {
                    (
                        format!("__wizer_global_{}", i).into(),
                        EntityType::Global(g),
                    )
                })
                .chain(self.defined_memories(cx).enumerate().map(|(i, (_, m))| {
                    (
                        format!("__wizer_memory_{}", i).into(),
                        EntityType::Memory(m),
                    )
                }))
                .chain(instance_types.iter().enumerate().map(|(i, ty)| {
                    (
                        format!("__wizer_instance_{}", i).into(),
                        EntityType::Instance(*ty),
                    )
                }))
                .collect(),
        }))
    }

    /// Get the count of how many instance imports are in each import section in
    /// this module.
    pub fn instance_import_counts<'b>(self, cx: &'b ModuleContext<'_>) -> &'b [u32] {
        &cx.defined(self).instance_import_counts
    }

    /// Get the aliases defined in this module.
    pub fn aliases<'a, 'b>(self, cx: &'b ModuleContext<'a>) -> &'b [wasmparser::Alias<'a>] {
        &cx.defined(self).aliases
    }

    /// Get an export from the `n`th instance by name.
    pub fn instance_export<'b>(
        self,
        cx: &'b ModuleContext<'_>,
        instance: u32,
        name: &str,
    ) -> Option<&'b EntityType> {
        let instance = usize::try_from(instance).unwrap();
        let info = cx.defined(self);
        let type_id = info.instances[instance];
        let instance = match cx.types.get(type_id) {
            Type::Instance(i) => i,
            _ => unreachable!(),
        };
        instance.exports.get(name)
    }

    /// Do a pre-order traversal over this module tree.
    pub fn pre_order<'a, F>(self, cx: &mut ModuleContext<'a>, mut f: F)
    where
        F: FnMut(&mut ModuleContext<'a>, Module),
    {
        let mut stack = vec![self];
        while let Some(module) = stack.pop() {
            f(cx, module);
            let info = cx.defined(module);
            stack.extend(info.modules.iter().copied().rev());
        }
    }

    /// Get the first index in the memory space where a memory is defined rather
    /// than aliased or imported.
    pub fn defined_memories_index(self, cx: &ModuleContext) -> Option<u32> {
        cx.defined(self).defined_memories_index
    }

    /// The number of defined memories in this module.
    pub fn defined_memories_len(self, cx: &ModuleContext) -> usize {
        let info = cx.defined(self);
        info.defined_memories_index.map_or(0, |n| {
            let n = usize::try_from(n).unwrap();
            assert!(info.memories.len() > n);
            info.memories.len() - n
        })
    }

    /// Iterate over the defined memories in this module.
    pub fn defined_memories<'b>(
        self,
        cx: &'b ModuleContext<'_>,
    ) -> impl Iterator<Item = (u32, wasmparser::MemoryType)> + 'b {
        let info = cx.defined(self);
        info.memories
            .iter()
            .copied()
            .enumerate()
            .skip(
                info.defined_memories_index
                    .map_or(info.memories.len(), |i| usize::try_from(i).unwrap()),
            )
            .map(|(i, m)| (u32::try_from(i).unwrap(), m))
    }

    /// Get the first index in the global space where a global is defined rather
    /// than aliased or imported.
    pub fn defined_globals_index(self, cx: &ModuleContext) -> Option<u32> {
        cx.defined(self).defined_globals_index
    }

    /// The number of defined globals in this module.
    pub fn defined_globals_len(self, cx: &ModuleContext<'_>) -> usize {
        let info = cx.defined(self);
        info.defined_globals_index.map_or(0, |n| {
            let n = usize::try_from(n).unwrap();
            assert!(info.globals.len() > n);
            info.globals.len() - n
        })
    }

    /// Iterate over the defined globals in this module.
    pub fn defined_globals<'b>(
        self,
        cx: &'b ModuleContext<'_>,
    ) -> impl Iterator<Item = (u32, wasmparser::GlobalType)> + 'b {
        let info = cx.defined(self);
        info.globals
            .iter()
            .copied()
            .enumerate()
            .skip(
                info.defined_globals_index
                    .map_or(info.globals.len(), |i| usize::try_from(i).unwrap()),
            )
            .map(|(i, g)| (u32::try_from(i).unwrap(), g))
    }

    /// Iterate over the initial sections in this Wasm module.
    pub fn initial_sections<'a, 'b>(
        self,
        cx: &'b ModuleContext<'a>,
    ) -> impl Iterator<Item = &'b wasm_encoder::RawSection<'a>> + 'b {
        let info = cx.defined(self);
        info.raw_sections
            .iter()
            .filter(|s| s.id != SectionId::Custom.into())
            .take_while(|s| match s.id {
                x if x == SectionId::Type.into() => true,
                x if x == SectionId::Import.into() => true,
                x if x == SectionId::Alias.into() => true,
                x if x == SectionId::Module.into() => true,
                x if x == SectionId::Instance.into() => true,
                _ => false,
            })
    }

    /// Get a slice of this module's original raw sections.
    pub fn raw_sections<'a, 'b>(
        self,
        cx: &'b ModuleContext<'a>,
    ) -> &'b [wasm_encoder::RawSection<'a>] {
        &cx.defined(self).raw_sections
    }

    /// Get a slice of this module's nested child modules.
    pub fn child_modules<'b>(self, cx: &'b ModuleContext<'_>) -> &'b [Module] {
        &cx.defined(self).modules
    }

    /// Get a slice of this module's exports.
    pub fn exports<'a, 'b>(self, cx: &'b ModuleContext<'a>) -> &'b [wasmparser::Export<'a>] {
        &cx.defined(self).exports
    }

    /// Get a slice of this module's imports.
    pub fn imports<'a, 'b>(self, cx: &'b ModuleContext<'a>) -> &'b [wasmparser::Import<'a>] {
        &cx.defined(self).imports
    }

    /// Get this module's defined (as opposed to imported or aliased)
    /// instantiations.
    ///
    /// The return value maps an instance index to the module that was
    /// instantiated and the associated instantiation arguments.
    pub fn instantiations<'a, 'b>(
        self,
        cx: &'b ModuleContext<'a>,
    ) -> &'b BTreeMap<u32, (Module, Vec<wasmparser::InstanceArg<'a>>)> {
        &cx.defined(self).instantiations
    }

    /// Get this module's `n`th nested child module.
    pub fn child_module_at(self, cx: &ModuleContext<'_>, n: u32) -> Module {
        cx.defined(self).modules[usize::try_from(n).unwrap()]
    }

    /// Get the full types index space for this module.
    pub fn types<'a, 'b>(self, cx: &'b ModuleContext<'a>) -> &'b [TypeId] {
        &cx.defined(self).types
    }

    /// Get the type at the given index.
    ///
    /// Panics if the types index space does not contain the given index.
    pub fn type_at<'a, 'b>(self, cx: &'b ModuleContext<'a>, type_index: u32) -> &'b Type<'a> {
        let id = self.type_id_at(cx, type_index);
        cx.types.get(id)
    }

    /// Get the type at the given index.
    ///
    /// Panics if the types index space does not contain the given index.
    pub fn type_id_at(self, cx: &ModuleContext<'_>, type_index: u32) -> TypeId {
        cx.defined(self).types[usize::try_from(type_index).unwrap()]
    }

    /// Get the id for instance type at the given type index.
    ///
    /// Panics if the types index space does not contain the given index or the
    /// type at the index is not an instance type.
    pub fn instance_type_at<'a, 'b>(self, cx: &'b ModuleContext<'a>, type_index: u32) -> TypeId {
        if let Type::Instance(_) = self.type_at(cx, type_index) {
            self.types(cx)[usize::try_from(type_index).unwrap()]
        } else {
            panic!("not an instance type")
        }
    }
}
