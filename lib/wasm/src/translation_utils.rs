///! Helper functions and structures for the translation.
use wasmparser;
use cretonne;
use std::u32;
use code_translator;
use module_translator;

/// Index of a function (imported or defined) inside the WebAssembly module.
pub type FunctionIndex = usize;
/// Index of a table (imported or defined) inside the WebAssembly module.
pub type TableIndex = usize;
/// Index of a global variable (imported or defined) inside the WebAssembly module.
pub type GlobalIndex = usize;
/// Index of a linear memory (imported or defined) inside the WebAssembly module.
pub type MemoryIndex = usize;
/// Index of a signature (imported or defined) inside the WebAssembly module.
pub type SignatureIndex = usize;
/// Raw byte read from memory.
pub type RawByte = u8;
/// Pointer referring to a memory address.
pub type MemoryAddress = usize;

/// WebAssembly import.
#[derive(Debug, Clone, Copy)]
pub enum Import {
    Function { sig_index: u32 },
    Memory(Memory),
    Global(Global),
    Table(Table),
}

/// WebAssembly global.
#[derive(Debug, Clone, Copy)]
pub struct Global {
    /// The type of the value stored in the global.
    pub ty: cretonne::ir::Type,
    /// A flag indicating whether the value may change at runtime.
    pub mutability: bool,
    /// The source of the initial value.
    pub initializer: GlobalInit,
}

/// Globals are initialized via the four `const` operators or by referring to another import.
#[derive(Debug, Clone, Copy)]
pub enum GlobalInit {
    /// An `i32.const`.
    I32Const(i32),
    /// An `i64.const`.
    I64Const(i64),
    /// An `f32.const`.
    F32Const(u32),
    /// An `f64.const`.
    F64Const(u64),
    /// A `get_global` of another global.
    GlobalRef(GlobalIndex),
    ///< The global is imported from, and thus initialized by, a different module.
    Import(),
}

/// WebAssembly table.
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// The type of data stored in elements of the table.
    pub ty: TableElementType,
    /// The minimum number of elements in the table.
    pub size: usize,
    /// The maximum number of elements in the table.
    pub maximum: Option<usize>,
}

/// WebAssembly table element. Can be a function or a scalar type.
#[derive(Debug, Clone, Copy)]
pub enum TableElementType {
    Val(cretonne::ir::Type),
    Func(),
}

/// WebAssembly linear memory.
#[derive(Debug, Clone, Copy)]
pub struct Memory {
    /// The minimum number of pages in the memory.
    pub pages_count: usize,
    /// The maximum number of pages in the memory.
    pub maximum: Option<usize>,
}

/// Wrapper to a `get_local` and `set_local` index. They are WebAssembly's non-SSA variables.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Local(pub u32);
impl cretonne::entity::EntityRef for Local {
    fn new(index: usize) -> Self {
        debug_assert!(index < (u32::MAX as usize));
        Local(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}
impl Default for Local {
    fn default() -> Self {
        Local(u32::MAX)
    }
}

/// Helper function translating wasmparser types to Cretonne types when possible.
pub fn type_to_type(ty: &wasmparser::Type) -> Result<cretonne::ir::Type, ()> {
    match *ty {
        wasmparser::Type::I32 => Ok(cretonne::ir::types::I32),
        wasmparser::Type::I64 => Ok(cretonne::ir::types::I64),
        wasmparser::Type::F32 => Ok(cretonne::ir::types::F32),
        wasmparser::Type::F64 => Ok(cretonne::ir::types::F64),
        _ => Err(()),
    }
}

/// Turns a `wasmparser` `f32` into a `Cretonne` one.
pub fn f32_translation(x: wasmparser::Ieee32) -> cretonne::ir::immediates::Ieee32 {
    cretonne::ir::immediates::Ieee32::with_bits(x.bits())
}

/// Turns a `wasmparser` `f64` into a `Cretonne` one.
pub fn f64_translation(x: wasmparser::Ieee64) -> cretonne::ir::immediates::Ieee64 {
    cretonne::ir::immediates::Ieee64::with_bits(x.bits())
}

/// Translate a `wasmparser` type into its `Cretonne` equivalent, when possible
pub fn translate_type(ty: wasmparser::Type) -> Result<Vec<cretonne::ir::Type>, ()> {
    match ty {
        wasmparser::Type::EmptyBlockType => Ok(Vec::new()),
        wasmparser::Type::I32 => Ok(vec![cretonne::ir::types::I32]),
        wasmparser::Type::F32 => Ok(vec![cretonne::ir::types::F32]),
        wasmparser::Type::I64 => Ok(vec![cretonne::ir::types::I64]),
        wasmparser::Type::F64 => Ok(vec![cretonne::ir::types::F64]),
        _ => panic!("unsupported return value type"),
    }
}

/// Inverts the key-value relation in the imports hashmap. Indeed, these hashmaps are built by
/// feeding the function indexes in the module but are used by the runtime with the `FuncRef` as
/// keys.
pub fn invert_hashmaps(
    imports: &code_translator::FunctionImports,
) -> module_translator::ImportMappings {
    let mut new_imports = module_translator::ImportMappings::new();
    for (func_index, func_ref) in &imports.functions {
        new_imports.functions.insert(*func_ref, *func_index);
    }
    for (sig_index, sig_ref) in &imports.signatures {
        new_imports.signatures.insert(*sig_ref, *sig_index);
    }
    new_imports
}
