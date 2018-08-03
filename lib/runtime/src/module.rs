//! A `Module` contains all the relevant information translated from a
//! WebAssembly module, except for the function bodies and data initializers.

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
