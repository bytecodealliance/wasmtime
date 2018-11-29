//! Data structures for representing decoded wasm modules.

use cranelift_codegen::ir;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{
    DefinedFuncIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, SignatureIndex, Table,
    TableIndex,
};
use std::cmp;
use std::collections::HashMap;
use std::string::String;
use std::vec::Vec;
use tunables::Tunables;

/// A WebAssembly table initializer.
#[derive(Clone, Debug)]
pub struct TableElements {
    /// The index of a table to initialize.
    pub table_index: TableIndex,
    /// Optionally, a global variable giving a base index.
    pub base: Option<GlobalIndex>,
    /// The offset to add to the base.
    pub offset: usize,
    /// The values to write into the table elements.
    pub elements: Vec<FuncIndex>,
}

/// An entity to export.
#[derive(Clone, Debug)]
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
#[derive(Debug, Clone)]
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
    pub fn for_memory(memory: Memory, tunables: &Tunables) -> Self {
        if let Some(maximum) = memory.maximum {
            // A heap with a declared maximum is prepared to be used with
            // threads and therefore be immovable, so make it static.
            MemoryStyle::Static {
                bound: cmp::max(tunables.static_memory_bound, maximum),
            }
        } else {
            // A heap without a declared maximum is likely to want to be small
            // at least some of the time, so make it dynamic.
            MemoryStyle::Dynamic
        }
    }
}

/// A WebAssembly linear memory description along with our chosen style for
/// implementing it.
#[derive(Debug)]
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
        Self {
            memory,
            style: MemoryStyle::for_memory(memory, tunables),
            offset_guard_size: tunables.offset_guard_size,
        }
    }
}

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
#[derive(Debug)]
pub struct Module {
    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    pub signatures: PrimaryMap<SignatureIndex, ir::Signature>,

    /// Names of imported functions.
    pub imported_funcs: Vec<(String, String)>,

    /// Types of functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, SignatureIndex>,

    /// WebAssembly tables.
    pub tables: PrimaryMap<TableIndex, Table>,

    /// WebAssembly linear memory plans.
    pub memory_plans: PrimaryMap<MemoryIndex, MemoryPlan>,

    /// WebAssembly global variables.
    pub globals: PrimaryMap<GlobalIndex, Global>,

    /// Exported entities.
    pub exports: HashMap<String, Export>,

    /// The module "start" function, if present.
    pub start_func: Option<FuncIndex>,

    /// WebAssembly table initializers.
    pub table_elements: Vec<TableElements>,
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Self {
            signatures: PrimaryMap::new(),
            imported_funcs: Vec::new(),
            functions: PrimaryMap::new(),
            tables: PrimaryMap::new(),
            memory_plans: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            exports: HashMap::new(),
            start_func: None,
            table_elements: Vec::new(),
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
}

/// A data initializer for linear memory.
pub struct DataInitializer<'data> {
    /// The index of the memory to initialize.
    pub memory_index: MemoryIndex,
    /// Optionally a globalvar base to initialize at.
    pub base: Option<GlobalIndex>,
    /// A constant offset to initialize at.
    pub offset: usize,
    /// The initialization data.
    pub data: &'data [u8],
}

/// References to the input wasm data buffer to be decoded and processed later,
/// separately from the main module translation.
pub struct LazyContents<'data> {
    /// References to the function bodies.
    pub function_body_inputs: PrimaryMap<DefinedFuncIndex, &'data [u8]>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,
}

impl<'data> LazyContents<'data> {
    pub fn new() -> Self {
        Self {
            function_body_inputs: PrimaryMap::new(),
            data_initializers: Vec::new(),
        }
    }
}
