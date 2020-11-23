//! Data structures for representing decoded wasm modules.

use crate::tunables::Tunables;
use crate::WASM_MAX_PAGES;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{
    DataIndex, DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex,
    ElemIndex, EntityIndex, EntityType, FuncIndex, Global, GlobalIndex, InstanceIndex, Memory,
    MemoryIndex, ModuleIndex, SignatureIndex, Table, TableIndex, TypeIndex, WasmFuncType,
};
use indexmap::IndexMap;
use more_asserts::assert_ge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Arc,
};

/// A WebAssembly table initializer.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct TableElements {
    /// The index of a table to initialize.
    pub table_index: TableIndex,
    /// Optionally, a global variable giving a base index.
    pub base: Option<GlobalIndex>,
    /// The offset to add to the base.
    pub offset: usize,
    /// The values to write into the table elements.
    pub elements: Box<[FuncIndex]>,
}

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
        // If the module doesn't declare an explicit maximum treat it as 4GiB.
        let maximum = memory.maximum.unwrap_or(WASM_MAX_PAGES);
        if maximum <= tunables.static_memory_bound {
            assert_ge!(tunables.static_memory_bound, memory.minimum);
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

/// Different types that can appear in a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleType {
    /// A function type, indexed further into the `signatures` table.
    Function(SignatureIndex),
    /// A module type
    Module {
        /// The module's imports
        imports: Vec<(String, Option<String>, EntityType)>,
        /// The module's exports
        exports: Vec<(String, EntityType)>,
    },
    /// An instance type
    Instance {
        /// the instance's exports
        exports: Vec<(String, EntityType)>,
    },
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// A unique identifier (within this process) for this module.
    #[serde(skip_serializing, skip_deserializing, default = "Module::next_id")]
    pub id: usize,

    /// The name of this wasm module, often found in the wasm file.
    pub name: Option<String>,

    /// All import records, in the order they are declared in the module.
    pub imports: Vec<(String, Option<String>, EntityIndex)>,

    /// Exported entities.
    pub exports: IndexMap<String, EntityIndex>,

    /// The module "start" function, if present.
    pub start_func: Option<FuncIndex>,

    /// WebAssembly table initializers.
    pub table_elements: Vec<TableElements>,

    /// WebAssembly passive elements.
    pub passive_elements: HashMap<ElemIndex, Box<[FuncIndex]>>,

    /// WebAssembly passive data segments.
    #[serde(with = "passive_data_serde")]
    pub passive_data: HashMap<DataIndex, Arc<[u8]>>,

    /// WebAssembly table initializers.
    pub func_names: HashMap<FuncIndex, String>,

    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    pub signatures: PrimaryMap<SignatureIndex, WasmFuncType>,

    /// Types declared in the wasm module.
    pub types: PrimaryMap<TypeIndex, ModuleType>,

    /// Number of imported functions in the module.
    pub num_imported_funcs: usize,

    /// Number of imported tables in the module.
    pub num_imported_tables: usize,

    /// Number of imported memories in the module.
    pub num_imported_memories: usize,

    /// Number of imported globals in the module.
    pub num_imported_globals: usize,

    /// Types of functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, SignatureIndex>,

    /// WebAssembly tables.
    pub table_plans: PrimaryMap<TableIndex, TablePlan>,

    /// WebAssembly linear memory plans.
    pub memory_plans: PrimaryMap<MemoryIndex, MemoryPlan>,

    /// WebAssembly global variables.
    pub globals: PrimaryMap<GlobalIndex, Global>,

    /// WebAssembly instances.
    pub instances: PrimaryMap<InstanceIndex, Instance>,

    /// WebAssembly modules.
    pub modules: PrimaryMap<ModuleIndex, TypeIndex>,
}

/// Different forms an instance can take in a wasm module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instance {
    /// This is an imported instance with the specified type
    Import(TypeIndex),
    /// This is a locally created instance which instantiates the specified
    /// module with the given list of entities.
    Instantiate {
        /// The module that this instance is instantiating.
        module: ModuleIndex,
        /// The arguments provided to instantiation.
        args: Vec<EntityIndex>,
    },
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Self {
            id: Self::next_id(),
            name: None,
            imports: Vec::new(),
            exports: IndexMap::new(),
            start_func: None,
            table_elements: Vec::new(),
            passive_elements: HashMap::new(),
            passive_data: HashMap::new(),
            func_names: HashMap::new(),
            num_imported_funcs: 0,
            num_imported_tables: 0,
            num_imported_memories: 0,
            num_imported_globals: 0,
            signatures: PrimaryMap::new(),
            functions: PrimaryMap::new(),
            table_plans: PrimaryMap::new(),
            memory_plans: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            instances: PrimaryMap::new(),
            modules: PrimaryMap::new(),
            types: PrimaryMap::new(),
        }
    }

    /// Get the given passive element, if it exists.
    pub fn get_passive_element(&self, index: ElemIndex) -> Option<&[FuncIndex]> {
        self.passive_elements.get(&index).map(|es| &**es)
    }

    fn next_id() -> usize {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        NEXT_ID.fetch_add(1, SeqCst)
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

    /// Convenience method for looking up the original Wasm signature of a
    /// function.
    pub fn wasm_func_type(&self, func_index: FuncIndex) -> &WasmFuncType {
        &self.signatures[self.functions[func_index]]
    }
}

impl Default for Module {
    fn default() -> Module {
        Module::new()
    }
}

mod passive_data_serde {
    use super::{Arc, DataIndex, HashMap};
    use serde::{de::MapAccess, de::Visitor, ser::SerializeMap, Deserializer, Serializer};
    use std::fmt;

    pub(super) fn serialize<S>(
        data: &HashMap<DataIndex, Arc<[u8]>>,
        ser: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = ser.serialize_map(Some(data.len()))?;
        for (k, v) in data {
            map.serialize_entry(k, v.as_ref())?;
        }
        map.end()
    }

    struct PassiveDataVisitor;
    impl<'de> Visitor<'de> for PassiveDataVisitor {
        type Value = HashMap<DataIndex, Arc<[u8]>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a passive_data map")
        }
        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));
            while let Some((key, value)) = access.next_entry::<_, Vec<u8>>()? {
                map.insert(key, value.into());
            }
            Ok(map)
        }
    }

    pub(super) fn deserialize<'de, D>(de: D) -> Result<HashMap<DataIndex, Arc<[u8]>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_map(PassiveDataVisitor)
    }
}
