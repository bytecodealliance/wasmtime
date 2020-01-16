//! Data structures for representing decoded wasm modules.

use crate::module_environ::FunctionBodyData;
use crate::tunables::Tunables;
use cranelift_codegen::ir;
use cranelift_entity::{EntityRef, PrimaryMap, SecondaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, Global,
    GlobalIndex, Memory, MemoryIndex, SignatureIndex, Table, TableIndex,
};
use indexmap::IndexMap;
use more_asserts::assert_ge;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A WebAssembly table initializer.
#[derive(Clone, Debug, Hash)]
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

/// An entity to export.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Export {
    /// Function export.
    Function(FuncIndex),
    /// Table export.
    Table(TableIndex),
    /// Memory export.
    Memory(MemoryIndex),
    /// Global export.
    Global(GlobalIndex),
}

/// Implemenation styles for WebAssembly linear memory.
#[derive(Debug, Clone, Hash)]
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
        if let Some(maximum) = memory.maximum {
            if maximum <= tunables.static_memory_bound {
                // A heap with a declared maximum can be immovable, so make
                // it static.
                assert_ge!(tunables.static_memory_bound, memory.minimum);
                return (
                    Self::Static {
                        bound: tunables.static_memory_bound,
                    },
                    tunables.static_memory_offset_guard_size,
                );
            }
        }

        // Otherwise, make it dynamic.
        (Self::Dynamic, tunables.dynamic_memory_offset_guard_size)
    }
}

/// A WebAssembly linear memory description along with our chosen style for
/// implementing it.
#[derive(Debug, Clone, Hash)]
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
#[derive(Debug, Clone, Hash)]
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
#[derive(Debug, Clone, Hash)]
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

/// Allows module strings to be cached as reused across
/// multiple threads. Useful for debug/trace information.
#[derive(Debug, Clone)]
pub struct ModuleSyncString(Option<Arc<String>>);

impl ModuleSyncString {
    /// Gets optional string reference.
    pub fn get(&self) -> Option<&str> {
        self.0.as_deref().map(|s| s.as_str())
    }
    /// Constructs the string.
    pub fn new(s: Option<&str>) -> Self {
        ModuleSyncString(s.map(|s| Arc::new(s.to_string())))
    }
}

impl Default for ModuleSyncString {
    fn default() -> Self {
        ModuleSyncString(None)
    }
}

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
// WARNING: when modifying, make sure that `hash_for_cache` is still valid!
#[derive(Debug)]
pub struct Module {
    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    pub signatures: PrimaryMap<SignatureIndex, ir::Signature>,

    /// Names of imported functions, as well as the index of the import that
    /// performed this import.
    pub imported_funcs: PrimaryMap<FuncIndex, (String, String, u32)>,

    /// Names of imported tables.
    pub imported_tables: PrimaryMap<TableIndex, (String, String, u32)>,

    /// Names of imported memories.
    pub imported_memories: PrimaryMap<MemoryIndex, (String, String, u32)>,

    /// Names of imported globals.
    pub imported_globals: PrimaryMap<GlobalIndex, (String, String, u32)>,

    /// Types of functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, SignatureIndex>,

    /// WebAssembly tables.
    pub table_plans: PrimaryMap<TableIndex, TablePlan>,

    /// WebAssembly linear memory plans.
    pub memory_plans: PrimaryMap<MemoryIndex, MemoryPlan>,

    /// WebAssembly global variables.
    pub globals: PrimaryMap<GlobalIndex, Global>,

    /// Exported entities.
    pub exports: IndexMap<String, Export>,

    /// The module "start" function, if present.
    pub start_func: Option<FuncIndex>,

    /// WebAssembly table initializers.
    pub table_elements: Vec<TableElements>,

    /// Module name.
    pub name: ModuleSyncString,

    /// Function names.
    pub func_names: SecondaryMap<FuncIndex, ModuleSyncString>,
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Self {
            signatures: PrimaryMap::new(),
            imported_funcs: PrimaryMap::new(),
            imported_tables: PrimaryMap::new(),
            imported_memories: PrimaryMap::new(),
            imported_globals: PrimaryMap::new(),
            functions: PrimaryMap::new(),
            table_plans: PrimaryMap::new(),
            memory_plans: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            exports: IndexMap::new(),
            start_func: None,
            table_elements: Vec::new(),
            name: Default::default(),
            func_names: SecondaryMap::new(),
        }
    }

    /// Convert a `DefinedFuncIndex` into a `FuncIndex`.
    pub fn func_index(&self, defined_func: DefinedFuncIndex) -> FuncIndex {
        FuncIndex::new(self.imported_funcs.len() + defined_func.index())
    }

    /// Convert a `FuncIndex` into a `DefinedFuncIndex`. Returns None if the
    /// index is an imported function.
    pub fn defined_func_index(&self, func: FuncIndex) -> Option<DefinedFuncIndex> {
        if func.index() < self.imported_funcs.len() {
            None
        } else {
            Some(DefinedFuncIndex::new(
                func.index() - self.imported_funcs.len(),
            ))
        }
    }

    /// Test whether the given function index is for an imported function.
    pub fn is_imported_function(&self, index: FuncIndex) -> bool {
        index.index() < self.imported_funcs.len()
    }

    /// Convert a `DefinedTableIndex` into a `TableIndex`.
    pub fn table_index(&self, defined_table: DefinedTableIndex) -> TableIndex {
        TableIndex::new(self.imported_tables.len() + defined_table.index())
    }

    /// Convert a `TableIndex` into a `DefinedTableIndex`. Returns None if the
    /// index is an imported table.
    pub fn defined_table_index(&self, table: TableIndex) -> Option<DefinedTableIndex> {
        if table.index() < self.imported_tables.len() {
            None
        } else {
            Some(DefinedTableIndex::new(
                table.index() - self.imported_tables.len(),
            ))
        }
    }

    /// Test whether the given table index is for an imported table.
    pub fn is_imported_table(&self, index: TableIndex) -> bool {
        index.index() < self.imported_tables.len()
    }

    /// Convert a `DefinedMemoryIndex` into a `MemoryIndex`.
    pub fn memory_index(&self, defined_memory: DefinedMemoryIndex) -> MemoryIndex {
        MemoryIndex::new(self.imported_memories.len() + defined_memory.index())
    }

    /// Convert a `MemoryIndex` into a `DefinedMemoryIndex`. Returns None if the
    /// index is an imported memory.
    pub fn defined_memory_index(&self, memory: MemoryIndex) -> Option<DefinedMemoryIndex> {
        if memory.index() < self.imported_memories.len() {
            None
        } else {
            Some(DefinedMemoryIndex::new(
                memory.index() - self.imported_memories.len(),
            ))
        }
    }

    /// Test whether the given memory index is for an imported memory.
    pub fn is_imported_memory(&self, index: MemoryIndex) -> bool {
        index.index() < self.imported_memories.len()
    }

    /// Convert a `DefinedGlobalIndex` into a `GlobalIndex`.
    pub fn global_index(&self, defined_global: DefinedGlobalIndex) -> GlobalIndex {
        GlobalIndex::new(self.imported_globals.len() + defined_global.index())
    }

    /// Convert a `GlobalIndex` into a `DefinedGlobalIndex`. Returns None if the
    /// index is an imported global.
    pub fn defined_global_index(&self, global: GlobalIndex) -> Option<DefinedGlobalIndex> {
        if global.index() < self.imported_globals.len() {
            None
        } else {
            Some(DefinedGlobalIndex::new(
                global.index() - self.imported_globals.len(),
            ))
        }
    }

    /// Test whether the given global index is for an imported global.
    pub fn is_imported_global(&self, index: GlobalIndex) -> bool {
        index.index() < self.imported_globals.len()
    }

    /// Computes hash of the module for the purpose of caching.
    pub fn hash_for_cache<'data, H>(
        &self,
        function_body_inputs: &PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        state: &mut H,
    ) where
        H: Hasher,
    {
        // There's no need to cache names (strings), start function
        // and data initializers (for both memory and tables)
        self.signatures.hash(state);
        self.functions.hash(state);
        self.table_plans.hash(state);
        self.memory_plans.hash(state);
        self.globals.hash(state);
        // IndexMap (self.export) iterates over values in order of item inserts
        // Let's actually sort the values.
        let mut exports = self.exports.values().collect::<Vec<_>>();
        exports.sort();
        for val in exports {
            val.hash(state);
        }
        function_body_inputs.hash(state);
    }
}
