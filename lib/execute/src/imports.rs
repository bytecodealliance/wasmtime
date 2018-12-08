use cranelift_entity::PrimaryMap;
use cranelift_wasm::{FuncIndex, GlobalIndex, MemoryIndex, TableIndex};
use vmcontext::{VMFunctionBody, VMGlobal, VMMemory, VMTable};

/// Resolved import pointers.
#[derive(Debug)]
pub struct Imports {
    /// Resolved addresses for imported functions.
    pub functions: PrimaryMap<FuncIndex, *const VMFunctionBody>,

    /// Resolved addresses for imported tables.
    pub tables: PrimaryMap<TableIndex, *mut VMTable>,

    /// Resolved addresses for imported globals.
    pub globals: PrimaryMap<GlobalIndex, *mut VMGlobal>,

    /// Resolved addresses for imported memories.
    pub memories: PrimaryMap<MemoryIndex, *mut VMMemory>,
}

impl Imports {
    pub fn new() -> Self {
        Self {
            functions: PrimaryMap::new(),
            tables: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            memories: PrimaryMap::new(),
        }
    }
}
