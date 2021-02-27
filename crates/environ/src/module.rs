//! Data structures for representing decoded wasm modules.

use crate::tunables::Tunables;
use crate::{DataInitializer, WASM_MAX_PAGES, WASM_PAGE_SIZE};
use cranelift_codegen::ir;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Implemenation styles for WebAssembly linear memory.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum MemoryStyle {
    /// The actual memory can be resized and moved.
    Dynamic,
    /// Addresss space is allocated up front.
    Static {
        /// The number of mapped and unmapped pages.
        bound: u32,
    },
}

impl MemoryStyle {
    /// Decide on an implementation style for the given `Memory`.
    pub fn for_memory(memory: Memory, tunables: &Tunables) -> (Self, u64) {
        // A heap with a maximum that doesn't exceed the static memory bound specified by the
        // tunables make it static.
        //
        // If the module doesn't declare an explicit maximum treat it as 4GiB when not
        // requested to use the static memory bound itself as the maximum.
        let maximum = std::cmp::min(
            memory.maximum.unwrap_or(WASM_MAX_PAGES),
            if tunables.static_memory_bound_is_maximum {
                std::cmp::min(tunables.static_memory_bound, WASM_MAX_PAGES)
            } else {
                WASM_MAX_PAGES
            },
        );

        // Ensure the minimum is less than the maximum; the minimum might exceed the maximum
        // when the memory is artificially bounded via `static_memory_bound_is_maximum` above
        if memory.minimum <= maximum && maximum <= tunables.static_memory_bound {
            return (
                Self::Static {
                    bound: tunables.static_memory_bound,
                },
                tunables.static_memory_offset_guard_size,
            );
        }

        // Otherwise, make it dynamic.
        (Self::Dynamic, tunables.dynamic_memory_offset_guard_size)
    }
}

/// A WebAssembly linear memory description along with our chosen style for
/// implementing it.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct MemoryPlan {
    /// The WebAssembly linear memory description.
    pub memory: Memory,
    /// Our chosen implementation style.
    pub style: MemoryStyle,
    /// Our chosen offset-guard size.
    pub offset_guard_size: u64,
}

impl MemoryPlan {
    /// Draw up a plan for implementing a `Memory`.
    pub fn for_memory(memory: Memory, tunables: &Tunables) -> Self {
        let (style, offset_guard_size) = MemoryStyle::for_memory(memory, tunables);
        Self {
            memory,
            style,
            offset_guard_size,
        }
    }
}

/// A WebAssembly linear memory initializer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryInitializer {
    /// The index of a linear memory to initialize.
    pub memory_index: MemoryIndex,
    /// Optionally, a global variable giving a base index.
    pub base: Option<GlobalIndex>,
    /// The offset to add to the base.
    pub offset: usize,
    /// The data to write into the linear memory.
    pub data: Box<[u8]>,
}

impl From<DataInitializer<'_>> for MemoryInitializer {
    fn from(initializer: DataInitializer) -> Self {
        Self {
            memory_index: initializer.memory_index,
            base: initializer.base,
            offset: initializer.offset,
            data: initializer.data.into(),
        }
    }
}

/// The type of WebAssembly linear memory initialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryInitialization {
    /// Memory initialization is paged.
    ///
    /// To be paged, the following requirements must be met:
    ///
    /// * All data segments must reference defined memories.
    /// * All data segments must not use a global base.
    /// * All data segments must be in bounds.
    ///
    /// Paged initialization is performed by memcopying individual pages to the linear memory.
    Paged {
        /// The size of each page stored in the map.
        /// This is expected to be the host page size.
        page_size: usize,
        /// The map of defined memory index to a list of page data.
        /// The list of page data is sparse, with None representing a zero page.
        /// The size of the list will be the maximum page written to by a data segment.
        map: PrimaryMap<DefinedMemoryIndex, Vec<Option<Box<[u8]>>>>,
    },
    /// Memory initialization is out of bounds.
    ///
    /// To be out of bounds, the following requirements must be met:
    ///
    /// * All data segments must reference defined memories.
    /// * All data segments must not use a global base.
    /// * At least one data segments was out of bounds.
    ///
    /// This can be used to quickly return an error when the module is instantiated.
    OutOfBounds,
    /// Memory initialization is segmented.
    ///
    /// To be segmented, at least one of the following requirements must be met:
    ///
    /// * A data segment referenced an imported memory.
    /// * A data segment uses a global base.
    ///
    /// Segmented initialization is performed by processing the complete set of data segments
    /// when the module is instantiated.
    ///
    /// This ensures that initialization side-effects are observed according to the bulk-memory proposal.
    Segmented(Box<[MemoryInitializer]>),
}

impl MemoryInitialization {
    /// Creates a new memory initialization for a module and its data initializers.
    pub fn new(module: &Module, initializers: Vec<DataInitializer>) -> Self {
        let page_size = region::page::size();
        let num_defined_memories = module.memory_plans.len() - module.num_imported_memories;
        let mut out_of_bounds = false;
        let mut memories = PrimaryMap::with_capacity(num_defined_memories);

        for _ in 0..num_defined_memories {
            memories.push(Vec::new());
        }

        for initializer in &initializers {
            match (
                module.defined_memory_index(initializer.memory_index),
                initializer.base.is_some(),
            ) {
                (None, _) | (_, true) => {
                    // If the initializer references an imported memory or uses a global base,
                    // the complete set of segments will need to be processed at module instantiation
                    return Self::Segmented(
                        initializers
                            .into_iter()
                            .map(Into::into)
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    );
                }
                (Some(index), false) => {
                    if out_of_bounds {
                        continue;
                    }

                    // Perform a bounds check on the segment
                    if (initializer.offset + initializer.data.len())
                        > ((module.memory_plans[initializer.memory_index].memory.minimum as usize)
                            * (WASM_PAGE_SIZE as usize))
                    {
                        out_of_bounds = true;
                        continue;
                    }

                    let pages = &mut memories[index];
                    let mut page_index = initializer.offset / page_size;
                    let mut page_offset = initializer.offset % page_size;
                    let mut data_offset = 0;
                    let mut data_remaining = initializer.data.len();

                    if data_remaining == 0 {
                        continue;
                    }

                    // Copy the initialization data by each page
                    loop {
                        if page_index >= pages.len() {
                            pages.resize(page_index + 1, None);
                        }

                        let page = pages[page_index]
                            .get_or_insert_with(|| vec![0; page_size].into_boxed_slice());
                        let len = std::cmp::min(data_remaining, page_size - page_offset);

                        page[page_offset..page_offset + len]
                            .copy_from_slice(&initializer.data[data_offset..(data_offset + len)]);

                        if len == data_remaining {
                            break;
                        }

                        page_index += 1;
                        page_offset = 0;
                        data_offset += len;
                        data_remaining -= len;
                    }
                }
            };
        }

        if out_of_bounds {
            Self::OutOfBounds
        } else {
            Self::Paged {
                page_size,
                map: memories,
            }
        }
    }
}

/// Implemenation styles for WebAssembly tables.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum TableStyle {
    /// Signatures are stored in the table and checked in the caller.
    CallerChecksSignature,
}

impl TableStyle {
    /// Decide on an implementation style for the given `Table`.
    pub fn for_table(_table: Table, _tunables: &Tunables) -> Self {
        Self::CallerChecksSignature
    }
}

/// A WebAssembly table description along with our chosen style for
/// implementing it.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct TablePlan {
    /// The WebAssembly table description.
    pub table: cranelift_wasm::Table,
    /// Our chosen implementation style.
    pub style: TableStyle,
}

impl TablePlan {
    /// Draw up a plan for implementing a `Table`.
    pub fn for_table(table: Table, tunables: &Tunables) -> Self {
        let style = TableStyle::for_table(table, tunables);
        Self { table, style }
    }
}

/// A WebAssembly table initializer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableInitializer {
    /// The index of a table to initialize.
    pub table_index: TableIndex,
    /// Optionally, a global variable giving a base index.
    pub base: Option<GlobalIndex>,
    /// The offset to add to the base.
    pub offset: usize,
    /// The values to write into the table elements.
    pub elements: Box<[FuncIndex]>,
}

/// Different types that can appear in a module.
///
/// Note that each of these variants are intended to index further into a
/// separate table.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ModuleType {
    Function(SignatureIndex),
    Module(ModuleTypeIndex),
    Instance(InstanceTypeIndex),
}

impl ModuleType {
    /// Asserts this is a `ModuleType::Function`, returning the underlying
    /// `SignatureIndex`.
    pub fn unwrap_function(&self) -> SignatureIndex {
        match self {
            ModuleType::Function(f) => *f,
            _ => panic!("not a function type"),
        }
    }
}

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// The name of this wasm module, often found in the wasm file.
    pub name: Option<String>,

    /// All import records, in the order they are declared in the module.
    pub initializers: Vec<Initializer>,

    /// Exported entities.
    pub exports: IndexMap<String, EntityIndex>,

    /// The module "start" function, if present.
    pub start_func: Option<FuncIndex>,

    /// WebAssembly table initializers.
    pub table_initializers: Vec<TableInitializer>,

    /// WebAssembly linear memory initializer.
    pub memory_initialization: Option<MemoryInitialization>,

    /// WebAssembly passive elements.
    pub passive_elements: Vec<Box<[FuncIndex]>>,

    /// The map from passive element index (element segment index space) to index in `passive_elements`.
    pub passive_elements_map: HashMap<ElemIndex, usize>,

    /// WebAssembly passive data segments.
    #[serde(with = "passive_data_serde")]
    pub passive_data: Vec<Arc<[u8]>>,

    /// The map from passive data index (data segment index space) to index in `passive_data`.
    pub passive_data_map: HashMap<DataIndex, usize>,

    /// WebAssembly function names.
    pub func_names: HashMap<FuncIndex, String>,

    /// Types declared in the wasm module.
    pub types: PrimaryMap<TypeIndex, ModuleType>,

    /// Number of imported or aliased functions in the module.
    pub num_imported_funcs: usize,

    /// Number of imported or aliased tables in the module.
    pub num_imported_tables: usize,

    /// Number of imported or aliased memories in the module.
    pub num_imported_memories: usize,

    /// Number of imported or aliased globals in the module.
    pub num_imported_globals: usize,

    /// Types of functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, SignatureIndex>,

    /// WebAssembly tables.
    pub table_plans: PrimaryMap<TableIndex, TablePlan>,

    /// WebAssembly linear memory plans.
    pub memory_plans: PrimaryMap<MemoryIndex, MemoryPlan>,

    /// WebAssembly global variables.
    pub globals: PrimaryMap<GlobalIndex, Global>,

    /// The type of each wasm instance this module defines.
    pub instances: PrimaryMap<InstanceIndex, InstanceTypeIndex>,

    /// The type of each nested wasm module this module contains.
    pub modules: PrimaryMap<ModuleIndex, ModuleTypeIndex>,
}

/// Initialization routines for creating an instance, encompassing imports,
/// modules, instances, aliases, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Initializer {
    /// An imported item is required to be provided.
    Import {
        /// Name of this import
        name: String,
        /// The field name projection of this import. When module-linking is
        /// enabled this is always `None`. Otherwise this is always `Some`.
        field: Option<String>,
        /// Where this import will be placed, which also has type information
        /// about the import.
        index: EntityIndex,
    },

    /// An export from a previously defined instance is being inserted into our
    /// index space.
    ///
    /// Note that when the module linking proposal is enabled two-level imports
    /// will implicitly desugar to this initializer.
    AliasInstanceExport {
        /// The instance that we're referencing.
        instance: InstanceIndex,
        /// Which export is being inserted into our index space.
        export: String,
    },

    /// A module is being instantiated with previously configured intializers
    /// as arguments.
    Instantiate {
        /// The module that this instance is instantiating.
        module: ModuleIndex,
        /// The arguments provided to instantiation, along with their name in
        /// the instance being instantiated.
        args: IndexMap<String, EntityIndex>,
    },

    /// A module is being created from a set of compiled artifacts.
    CreateModule {
        /// The index of the artifact that's being convereted into a module.
        artifact_index: usize,
        /// The list of artifacts that this module value will be inheriting.
        artifacts: Vec<usize>,
        /// The list of modules that this module value will inherit.
        modules: Vec<ModuleUpvar>,
    },

    /// A module is created from a closed-over-module value, defined when this
    /// module was created.
    DefineModule(usize),
}

/// Where module values can come from when creating a new module from a compiled
/// artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleUpvar {
    /// A module value is inherited from the module creating the new module.
    Inherit(usize),
    /// A module value comes from the instance-to-be-created module index space.
    Local(ModuleIndex),
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Module::default()
    }

    /// Get the given passive element, if it exists.
    pub fn get_passive_element(&self, index: ElemIndex) -> Option<&[FuncIndex]> {
        let index = *self.passive_elements_map.get(&index)?;
        Some(self.passive_elements[index].as_ref())
    }

    /// Convert a `DefinedFuncIndex` into a `FuncIndex`.
    pub fn func_index(&self, defined_func: DefinedFuncIndex) -> FuncIndex {
        FuncIndex::new(self.num_imported_funcs + defined_func.index())
    }

    /// Convert a `FuncIndex` into a `DefinedFuncIndex`. Returns None if the
    /// index is an imported function.
    pub fn defined_func_index(&self, func: FuncIndex) -> Option<DefinedFuncIndex> {
        if func.index() < self.num_imported_funcs {
            None
        } else {
            Some(DefinedFuncIndex::new(
                func.index() - self.num_imported_funcs,
            ))
        }
    }

    /// Test whether the given function index is for an imported function.
    pub fn is_imported_function(&self, index: FuncIndex) -> bool {
        index.index() < self.num_imported_funcs
    }

    /// Convert a `DefinedTableIndex` into a `TableIndex`.
    pub fn table_index(&self, defined_table: DefinedTableIndex) -> TableIndex {
        TableIndex::new(self.num_imported_tables + defined_table.index())
    }

    /// Convert a `TableIndex` into a `DefinedTableIndex`. Returns None if the
    /// index is an imported table.
    pub fn defined_table_index(&self, table: TableIndex) -> Option<DefinedTableIndex> {
        if table.index() < self.num_imported_tables {
            None
        } else {
            Some(DefinedTableIndex::new(
                table.index() - self.num_imported_tables,
            ))
        }
    }

    /// Test whether the given table index is for an imported table.
    pub fn is_imported_table(&self, index: TableIndex) -> bool {
        index.index() < self.num_imported_tables
    }

    /// Convert a `DefinedMemoryIndex` into a `MemoryIndex`.
    pub fn memory_index(&self, defined_memory: DefinedMemoryIndex) -> MemoryIndex {
        MemoryIndex::new(self.num_imported_memories + defined_memory.index())
    }

    /// Convert a `MemoryIndex` into a `DefinedMemoryIndex`. Returns None if the
    /// index is an imported memory.
    pub fn defined_memory_index(&self, memory: MemoryIndex) -> Option<DefinedMemoryIndex> {
        if memory.index() < self.num_imported_memories {
            None
        } else {
            Some(DefinedMemoryIndex::new(
                memory.index() - self.num_imported_memories,
            ))
        }
    }

    /// Test whether the given memory index is for an imported memory.
    pub fn is_imported_memory(&self, index: MemoryIndex) -> bool {
        index.index() < self.num_imported_memories
    }

    /// Convert a `DefinedGlobalIndex` into a `GlobalIndex`.
    pub fn global_index(&self, defined_global: DefinedGlobalIndex) -> GlobalIndex {
        GlobalIndex::new(self.num_imported_globals + defined_global.index())
    }

    /// Convert a `GlobalIndex` into a `DefinedGlobalIndex`. Returns None if the
    /// index is an imported global.
    pub fn defined_global_index(&self, global: GlobalIndex) -> Option<DefinedGlobalIndex> {
        if global.index() < self.num_imported_globals {
            None
        } else {
            Some(DefinedGlobalIndex::new(
                global.index() - self.num_imported_globals,
            ))
        }
    }

    /// Test whether the given global index is for an imported global.
    pub fn is_imported_global(&self, index: GlobalIndex) -> bool {
        index.index() < self.num_imported_globals
    }

    /// Returns an iterator of all the imports in this module, along with their
    /// module name, field name, and type that's being imported.
    pub fn imports(&self) -> impl Iterator<Item = (&str, Option<&str>, EntityType)> {
        self.initializers.iter().filter_map(move |i| match i {
            Initializer::Import { name, field, index } => {
                Some((name.as_str(), field.as_deref(), self.type_of(*index)))
            }
            _ => None,
        })
    }

    /// Returns the type of an item based on its index
    pub fn type_of(&self, index: EntityIndex) -> EntityType {
        match index {
            EntityIndex::Global(i) => EntityType::Global(self.globals[i]),
            EntityIndex::Table(i) => EntityType::Table(self.table_plans[i].table),
            EntityIndex::Memory(i) => EntityType::Memory(self.memory_plans[i].memory),
            EntityIndex::Function(i) => EntityType::Function(self.functions[i]),
            EntityIndex::Instance(i) => EntityType::Instance(self.instances[i]),
            EntityIndex::Module(i) => EntityType::Module(self.modules[i]),
        }
    }
}

/// All types which are recorded for the entirety of a translation.
///
/// Note that this is shared amongst all modules coming out of a translation
/// in the case of nested modules and the module linking proposal.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TypeTables {
    pub wasm_signatures: PrimaryMap<SignatureIndex, WasmFuncType>,
    pub native_signatures: PrimaryMap<SignatureIndex, ir::Signature>,
    pub module_signatures: PrimaryMap<ModuleTypeIndex, ModuleSignature>,
    pub instance_signatures: PrimaryMap<InstanceTypeIndex, InstanceSignature>,
}

/// The type signature of known modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSignature {
    /// All imports in this module, listed in order with their name and
    /// what type they're importing.
    pub imports: IndexMap<String, EntityType>,
    /// Exports are what an instance type conveys, so we go through an
    /// indirection over there.
    pub exports: InstanceTypeIndex,
}

/// The type signature of known instances.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstanceSignature {
    /// The name of what's being exported as well as its type signature.
    pub exports: IndexMap<String, EntityType>,
}

mod passive_data_serde {
    use super::Arc;
    use serde::{de::SeqAccess, de::Visitor, ser::SerializeSeq, Deserializer, Serializer};
    use std::fmt;

    pub(super) fn serialize<S>(data: &Vec<Arc<[u8]>>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = ser.serialize_seq(Some(data.len()))?;
        for v in data {
            seq.serialize_element(v.as_ref())?;
        }
        seq.end()
    }

    struct PassiveDataVisitor;
    impl<'de> Visitor<'de> for PassiveDataVisitor {
        type Value = Vec<Arc<[u8]>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a passive data sequence")
        }

        fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: SeqAccess<'de>,
        {
            let mut data = Vec::with_capacity(access.size_hint().unwrap_or(0));
            while let Some(value) = access.next_element::<Vec<u8>>()? {
                data.push(value.into());
            }
            Ok(data)
        }
    }

    pub(super) fn deserialize<'de, D>(de: D) -> Result<Vec<Arc<[u8]>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_seq(PassiveDataVisitor)
    }
}
