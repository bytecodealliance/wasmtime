//! Helper functions and structures for the translation.
use cranelift_codegen::ir;
use std::u32;
use wasmparser;

/// Index type of a function (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct FuncIndex(u32);
entity_impl!(FuncIndex);

/// Index type of a defined function inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct DefinedFuncIndex(u32);
entity_impl!(DefinedFuncIndex);

/// Index type of a table (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct TableIndex(u32);
entity_impl!(TableIndex);

/// Index type of a global variable (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct GlobalIndex(u32);
entity_impl!(GlobalIndex);

/// Index type of a linear memory (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct MemoryIndex(u32);
entity_impl!(MemoryIndex);

/// Index type of a signature (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SignatureIndex(u32);
entity_impl!(SignatureIndex);

/// WebAssembly global.
#[derive(Debug, Clone, Copy)]
pub struct Global {
    /// The type of the value stored in the global.
    pub ty: ir::Type,
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
    Val(ir::Type),
    Func(),
}

/// WebAssembly linear memory.
#[derive(Debug, Clone, Copy)]
pub struct Memory {
    /// The minimum number of pages in the memory.
    pub pages_count: usize,
    /// The maximum number of pages in the memory.
    pub maximum: Option<usize>,
    /// Whether the memory may be shared between multiple threads.
    pub shared: bool,
}

/// Helper function translating wasmparser types to Cranelift types when possible.
pub fn type_to_type(ty: wasmparser::Type) -> Result<ir::Type, ()> {
    Ok(match ty {
        wasmparser::Type::I32 => ir::types::I32,
        wasmparser::Type::I64 => ir::types::I64,
        wasmparser::Type::F32 => ir::types::F32,
        wasmparser::Type::F64 => ir::types::F64,
        _ => return Err(()),
    })
}

/// Turns a `wasmparser` `f32` into a `Cranelift` one.
pub fn f32_translation(x: wasmparser::Ieee32) -> ir::immediates::Ieee32 {
    ir::immediates::Ieee32::with_bits(x.bits())
}

/// Turns a `wasmparser` `f64` into a `Cranelift` one.
pub fn f64_translation(x: wasmparser::Ieee64) -> ir::immediates::Ieee64 {
    ir::immediates::Ieee64::with_bits(x.bits())
}

/// Translate a `wasmparser` type into its `Cranelift` equivalent, when possible
pub fn num_return_values(ty: wasmparser::Type) -> usize {
    match ty {
        wasmparser::Type::EmptyBlockType => 0,
        wasmparser::Type::I32
        | wasmparser::Type::F32
        | wasmparser::Type::I64
        | wasmparser::Type::F64 => 1,
        _ => panic!("unsupported return value type"),
    }
}
