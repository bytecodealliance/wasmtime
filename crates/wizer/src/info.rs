use std::convert::TryFrom;
use std::ops::Range;
use types_interner::{EntityType, TypeId, TypesInterner};
use wasm_encoder::SectionId;

pub mod types_interner;

/// A collection of info about modules within a module linking bundle.
pub(crate) struct ModuleContext<'a> {
    arena: Vec<ModuleInfo<'a>>,
    types: TypesInterner,
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
    pub fn types(&self) -> &TypesInterner {
        &self.types
    }

    /// Get a shared reference to the `DefinedModuleInfo` for this module,
    /// following through aliases.
    fn defined(&self, module: Module) -> &DefinedModuleInfo<'a> {
        match &self.arena[module.id] {
            ModuleInfo::Defined(d) => return d,
        }
    }

    /// Get an exclusive reference to the `DefinedModuleInfo` for this module.
    ///
    /// Does not resolve through aliases, because you shouldn't ever mutate
    /// aliased modules.
    fn defined_mut(&mut self, module: Module) -> &mut DefinedModuleInfo<'a> {
        match &mut self.arena[module.id] {
            ModuleInfo::Defined(d) => d,
        }
    }
}

enum ModuleInfo<'a> {
    Defined(DefinedModuleInfo<'a>),
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

    /// Types available in this module.
    ///
    /// We keep track of these for determining how many things we need to
    /// re-export for new instantiations and for inner module's aliases.
    types: Vec<TypeId>,

    /// Imports made by this module.
    imports: Vec<wasmparser::Import<'a>>,

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
    /// Translate the given `wasmparser` entity type into its interned
    /// representation using this module's types space.
    pub fn entity_type(self, cx: &ModuleContext<'_>, ty: wasmparser::TypeRef) -> EntityType {
        cx.types().entity_type(ty, &cx.defined(self).types)
    }

    /// Add a new raw section to this module info during parsing.
    pub fn add_raw_section<'a>(
        self,
        cx: &mut ModuleContext<'a>,
        id: SectionId,
        range: Range<usize>,
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
    pub fn push_type<'a>(self, cx: &mut ModuleContext<'a>, ty: wasmparser::CompositeType) {
        let types_space = match &cx.arena[self.id] {
            ModuleInfo::Defined(d) => &d.types,
        };
        let ty = cx.types.insert_wasmparser(ty, types_space);
        cx.defined_mut(self).types.push(ty);
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
            wasmparser::TypeRef::Memory(ty) => {
                self.push_imported_memory(cx, ty);
            }
            wasmparser::TypeRef::Global(ty) => {
                self.push_imported_global(cx, ty);
            }
            wasmparser::TypeRef::Func(ty_idx) => {
                let ty = self.type_id_at(cx, ty_idx);
                self.push_function(cx, ty);
            }
            wasmparser::TypeRef::Table(ty) => {
                self.push_table(cx, ty);
            }
            wasmparser::TypeRef::Tag(_) => {
                unreachable!("exceptions are unsupported; checked in validation")
            }
        }
    }

    /// Push an export into this module.
    pub fn push_export<'a>(self, cx: &mut ModuleContext<'a>, export: wasmparser::Export<'a>) {
        cx.defined_mut(self).exports.push(export);
    }

    /// Is this the root of the module linking bundle?
    pub fn is_root(self) -> bool {
        self.id == 0
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

    /// Get a slice of this module's original raw sections.
    pub fn raw_sections<'a, 'b>(
        self,
        cx: &'b ModuleContext<'a>,
    ) -> &'b [wasm_encoder::RawSection<'a>] {
        &cx.defined(self).raw_sections
    }

    /// Get a slice of this module's exports.
    pub fn exports<'a, 'b>(self, cx: &'b ModuleContext<'a>) -> &'b [wasmparser::Export<'a>] {
        &cx.defined(self).exports
    }

    /// Get the full types index space for this module.
    pub fn types<'a, 'b>(self, cx: &'b ModuleContext<'a>) -> &'b [TypeId] {
        &cx.defined(self).types
    }

    /// Get the type at the given index.
    ///
    /// Panics if the types index space does not contain the given index.
    pub fn type_id_at(self, cx: &ModuleContext<'_>, type_index: u32) -> TypeId {
        cx.defined(self).types[usize::try_from(type_index).unwrap()]
    }
}
