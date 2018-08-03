//! Data structures for representing decoded wasm modules.

use cranelift_codegen::ir;
use cranelift_wasm::{
    FunctionIndex, Global, GlobalIndex, Memory, MemoryIndex, SignatureIndex, Table, TableIndex,
};
use std::collections::HashMap;

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
    pub elements: Vec<FunctionIndex>,
}

/// An entity to export.
#[derive(Clone, Debug)]
pub enum Export {
    /// Function export.
    Function(FunctionIndex),
    /// Table export.
    Table(TableIndex),
    /// Memory export.
    Memory(MemoryIndex),
    /// Global export.
    Global(GlobalIndex),
}

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
#[derive(Debug)]
pub struct Module {
    /// Unprocessed signatures exactly as provided by `declare_signature()`.
    pub signatures: Vec<ir::Signature>,

    /// Names of imported functions.
    pub imported_funcs: Vec<(String, String)>,

    /// Types of functions, imported and local.
    pub functions: Vec<SignatureIndex>,

    /// WebAssembly tables.
    pub tables: Vec<Table>,

    /// WebAssembly linear memories.
    pub memories: Vec<Memory>,

    /// WebAssembly global variables.
    pub globals: Vec<Global>,

    /// Exported entities.
    pub exports: HashMap<String, Export>,

    /// The module "start" function, if present.
    pub start_func: Option<FunctionIndex>,

    /// WebAssembly table initializers.
    pub table_elements: Vec<TableElements>,
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Self {
            signatures: Vec::new(),
            imported_funcs: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            exports: HashMap::new(),
            start_func: None,
            table_elements: Vec::new(),
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
    pub function_body_inputs: Vec<&'data [u8]>,

    /// References to the data initializers.
    pub data_initializers: Vec<DataInitializer<'data>>,
}

impl<'data> LazyContents<'data> {
    pub fn new() -> Self {
        Self {
            function_body_inputs: Vec::new(),
            data_initializers: Vec::new(),
        }
    }
}
