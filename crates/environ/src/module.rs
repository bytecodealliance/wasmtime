//! Data structures for representing decoded wasm modules.

use crate::tunables::Tunables;
use crate::WASM_MAX_PAGES;
use cranelift_codegen::ir;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{
    DataIndex, DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex,
    ElemIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, SignatureIndex, Table,
    TableIndex,
};
use indexmap::IndexMap;
use more_asserts::assert_ge;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Arc,
};

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

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
#[derive(Debug)]
pub struct Module {
    /// A unique identifier (within this process) for this module.
    pub id: usize,

    /// The name of this wasm module, often found in the wasm file.
    pub name: Option<String>,

    /// Local information about a module which is the bare minimum necessary to
    /// translate a function body. This is derived as `Hash` whereas this module
    /// isn't, since it contains too much information needed to translate a
    /// function.
    pub local: ModuleLocal,

    /// Names of imported functions, as well as the index of the import that
    /// performed this import.
    pub imported_funcs: PrimaryMap<FuncIndex, (String, String, u32)>,

    /// Names of imported tables.
    pub imported_tables: PrimaryMap<TableIndex, (String, String, u32)>,

    /// Names of imported memories.
    pub imported_memories: PrimaryMap<MemoryIndex, (String, String, u32)>,

    /// Names of imported globals.
    pub imported_globals: PrimaryMap<GlobalIndex, (String, String, u32)>,

    /// Exported entities.
    pub exports: IndexMap<String, Export>,

    /// The module "start" function, if present.
    pub start_func: Option<FuncIndex>,

    /// WebAssembly table initializers.
    pub table_elements: Vec<TableElements>,

    /// WebAssembly passive elements.
    pub passive_elements: HashMap<ElemIndex, Box<[FuncIndex]>>,

    /// WebAssembly passive data segments.
    pub passive_data: HashMap<DataIndex, Arc<[u8]>>,

    /// WebAssembly table initializers.
    pub func_names: HashMap<FuncIndex, String>,
}

/// Local information known about a wasm module, the bare minimum necessary to
/// translate function bodies.
///
/// This is stored within a `Module` and it implements `Hash`, unlike `Module`,
/// and is used as part of the cache key when we load compiled modules from the
/// global cache.
#[derive(Debug, Hash)]
pub struct ModuleLocal {
    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    pub signatures: PrimaryMap<SignatureIndex, ir::Signature>,

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
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        Self {
            id: NEXT_ID.fetch_add(1, SeqCst),
            name: None,
            imported_funcs: PrimaryMap::new(),
            imported_tables: PrimaryMap::new(),
            imported_memories: PrimaryMap::new(),
            imported_globals: PrimaryMap::new(),
            exports: IndexMap::new(),
            start_func: None,
            table_elements: Vec::new(),
            passive_elements: HashMap::new(),
            passive_data: HashMap::new(),
            func_names: HashMap::new(),
            local: ModuleLocal {
                num_imported_funcs: 0,
                num_imported_tables: 0,
                num_imported_memories: 0,
                num_imported_globals: 0,
                signatures: PrimaryMap::new(),
                functions: PrimaryMap::new(),
                table_plans: PrimaryMap::new(),
                memory_plans: PrimaryMap::new(),
                globals: PrimaryMap::new(),
            },
        }
    }

    /// Get the given passive element, if it exists.
    pub fn get_passive_element(&self, index: ElemIndex) -> Option<&[FuncIndex]> {
        self.passive_elements.get(&index).map(|es| &**es)
    }
}

impl ModuleLocal {
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
}
