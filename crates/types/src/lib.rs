//! Internal dependency of Wasmtime and Cranelift that defines types for
//! WebAssembly.

pub use wasmparser;

use cranelift_entity::entity_impl;
use serde::{Deserialize, Serialize};
use std::fmt;

mod error;
pub use error::*;

/// WebAssembly value type -- equivalent of `wasmparser`'s Type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmType {
    /// I32 type
    I32,
    /// I64 type
    I64,
    /// F32 type
    F32,
    /// F64 type
    F64,
    /// V128 type
    V128,
    /// Reference type
    Ref(WasmRefType),
}

impl fmt::Display for WasmType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WasmType::I32 => write!(f, "i32"),
            WasmType::I64 => write!(f, "i64"),
            WasmType::F32 => write!(f, "f32"),
            WasmType::F64 => write!(f, "f64"),
            WasmType::V128 => write!(f, "v128"),
            WasmType::Ref(rt) => write!(f, "{rt}"),
        }
    }
}

/// WebAssembly reference type -- equivalent of `wasmparser`'s RefType
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmRefType {
    pub nullable: bool,
    pub heap_type: WasmHeapType,
}

impl WasmRefType {
    pub const EXTERNREF: WasmRefType = WasmRefType {
        nullable: true,
        heap_type: WasmHeapType::Extern,
    };
    pub const FUNCREF: WasmRefType = WasmRefType {
        nullable: true,
        heap_type: WasmHeapType::Func,
    };
}

impl fmt::Display for WasmRefType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::FUNCREF => write!(f, "funcref"),
            Self::EXTERNREF => write!(f, "externref"),
            _ => {
                if self.nullable {
                    write!(f, "(ref null {})", self.heap_type)
                } else {
                    write!(f, "(ref {})", self.heap_type)
                }
            }
        }
    }
}

/// WebAssembly heap type -- equivalent of `wasmparser`'s HeapType
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmHeapType {
    Func,
    Extern,
    // FIXME: the `SignatureIndex` payload here is not suitable given all the
    // contexts that this type is used within. For example the Engine in
    // wasmtime hashes this index which is not appropriate because the index is
    // not globally unique.
    //
    // This probably needs to become `WasmHeapType<T>` where all of translation
    // uses `WasmHeapType<SignatureIndex>` and all of engine-level "stuff"  uses
    // `WasmHeapType<VMSharedSignatureIndex>`. This `<T>` would need to be
    // propagated to quite a few locations though so it's left for a future
    // refactoring at this time.
    TypedFunc(SignatureIndex),
}

impl fmt::Display for WasmHeapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Func => write!(f, "func"),
            Self::Extern => write!(f, "extern"),
            Self::TypedFunc(i) => write!(f, "func_sig{}", i.as_u32()),
        }
    }
}

/// WebAssembly function type -- equivalent of `wasmparser`'s FuncType.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WasmFuncType {
    params: Box<[WasmType]>,
    externref_params_count: usize,
    returns: Box<[WasmType]>,
    externref_returns_count: usize,
}

impl WasmFuncType {
    #[inline]
    pub fn new(params: Box<[WasmType]>, returns: Box<[WasmType]>) -> Self {
        let externref_params_count = params
            .iter()
            .filter(|p| match **p {
                WasmType::Ref(rt) => rt.heap_type == WasmHeapType::Extern,
                _ => false,
            })
            .count();
        let externref_returns_count = returns
            .iter()
            .filter(|r| match **r {
                WasmType::Ref(rt) => rt.heap_type == WasmHeapType::Extern,
                _ => false,
            })
            .count();
        WasmFuncType {
            params,
            externref_params_count,
            returns,
            externref_returns_count,
        }
    }

    /// Function params types.
    #[inline]
    pub fn params(&self) -> &[WasmType] {
        &self.params
    }

    /// How many `externref`s are in this function's params?
    #[inline]
    pub fn externref_params_count(&self) -> usize {
        self.externref_params_count
    }

    /// Returns params types.
    #[inline]
    pub fn returns(&self) -> &[WasmType] {
        &self.returns
    }

    /// How many `externref`s are in this function's returns?
    #[inline]
    pub fn externref_returns_count(&self) -> usize {
        self.externref_returns_count
    }
}

/// Index type of a function (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct FuncIndex(u32);
entity_impl!(FuncIndex);

/// Index type of a defined function inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct DefinedFuncIndex(u32);
entity_impl!(DefinedFuncIndex);

/// Index type of a defined table inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct DefinedTableIndex(u32);
entity_impl!(DefinedTableIndex);

/// Index type of a defined memory inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct DefinedMemoryIndex(u32);
entity_impl!(DefinedMemoryIndex);

/// Index type of a defined memory inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct OwnedMemoryIndex(u32);
entity_impl!(OwnedMemoryIndex);

/// Index type of a defined global inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct DefinedGlobalIndex(u32);
entity_impl!(DefinedGlobalIndex);

/// Index type of a table (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct TableIndex(u32);
entity_impl!(TableIndex);

/// Index type of a global variable (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct GlobalIndex(u32);
entity_impl!(GlobalIndex);

/// Index type of a linear memory (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct MemoryIndex(u32);
entity_impl!(MemoryIndex);

/// Index type of a signature (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct SignatureIndex(u32);
entity_impl!(SignatureIndex);

/// Index type of a passive data segment inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct DataIndex(u32);
entity_impl!(DataIndex);

/// Index type of a passive element segment inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct ElemIndex(u32);
entity_impl!(ElemIndex);

/// Index type of a type inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct TypeIndex(u32);
entity_impl!(TypeIndex);

/// Index type of an event inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct TagIndex(u32);
entity_impl!(TagIndex);

/// Index into the global list of modules found within an entire component.
///
/// Module translations are saved on the side to get fully compiled after
/// the original component has finished being translated.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct StaticModuleIndex(u32);
entity_impl!(StaticModuleIndex);

/// An index of an entity.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum EntityIndex {
    /// Function index.
    Function(FuncIndex),
    /// Table index.
    Table(TableIndex),
    /// Memory index.
    Memory(MemoryIndex),
    /// Global index.
    Global(GlobalIndex),
}

impl From<FuncIndex> for EntityIndex {
    fn from(idx: FuncIndex) -> EntityIndex {
        EntityIndex::Function(idx)
    }
}

impl From<TableIndex> for EntityIndex {
    fn from(idx: TableIndex) -> EntityIndex {
        EntityIndex::Table(idx)
    }
}

impl From<MemoryIndex> for EntityIndex {
    fn from(idx: MemoryIndex) -> EntityIndex {
        EntityIndex::Memory(idx)
    }
}

impl From<GlobalIndex> for EntityIndex {
    fn from(idx: GlobalIndex) -> EntityIndex {
        EntityIndex::Global(idx)
    }
}

/// A type of an item in a wasm module where an item is typically something that
/// can be exported.
#[allow(missing_docs)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EntityType {
    /// A global variable with the specified content type
    Global(Global),
    /// A linear memory with the specified limits
    Memory(Memory),
    /// An event definition.
    Tag(Tag),
    /// A table with the specified element type and limits
    Table(Table),
    /// A function type where the index points to the type section and records a
    /// function signature.
    Function(SignatureIndex),
}

impl EntityType {
    /// Assert that this entity is a global
    pub fn unwrap_global(&self) -> &Global {
        match self {
            EntityType::Global(g) => g,
            _ => panic!("not a global"),
        }
    }

    /// Assert that this entity is a memory
    pub fn unwrap_memory(&self) -> &Memory {
        match self {
            EntityType::Memory(g) => g,
            _ => panic!("not a memory"),
        }
    }

    /// Assert that this entity is a tag
    pub fn unwrap_tag(&self) -> &Tag {
        match self {
            EntityType::Tag(g) => g,
            _ => panic!("not a tag"),
        }
    }

    /// Assert that this entity is a table
    pub fn unwrap_table(&self) -> &Table {
        match self {
            EntityType::Table(g) => g,
            _ => panic!("not a table"),
        }
    }

    /// Assert that this entity is a function
    pub fn unwrap_func(&self) -> SignatureIndex {
        match self {
            EntityType::Function(g) => *g,
            _ => panic!("not a func"),
        }
    }
}

/// A WebAssembly global.
///
/// Note that we record both the original Wasm type and the Cranelift IR type
/// used to represent it. This is because multiple different kinds of Wasm types
/// might be represented with the same Cranelift IR type. For example, both a
/// Wasm `i64` and a `funcref` might be represented with a Cranelift `i64` on
/// 64-bit architectures, and when GC is not required for func refs.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Global {
    /// The Wasm type of the value stored in the global.
    pub wasm_ty: crate::WasmType,
    /// A flag indicating whether the value may change at runtime.
    pub mutability: bool,
}

/// Globals are initialized via the `const` operators or by referring to another import.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum GlobalInit {
    /// An `i32.const`.
    I32Const(i32),
    /// An `i64.const`.
    I64Const(i64),
    /// An `f32.const`.
    F32Const(u32),
    /// An `f64.const`.
    F64Const(u64),
    /// A `vconst`.
    V128Const(u128),
    /// A `global.get` of another global.
    GetGlobal(GlobalIndex),
    /// A `ref.null`.
    RefNullConst,
    /// A `ref.func <index>`.
    RefFunc(FuncIndex),
}

/// WebAssembly table.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// The table elements' Wasm type.
    pub wasm_ty: WasmRefType,
    /// The minimum number of elements in the table.
    pub minimum: u32,
    /// The maximum number of elements in the table.
    pub maximum: Option<u32>,
}

/// WebAssembly linear memory.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    /// The minimum number of pages in the memory.
    pub minimum: u64,
    /// The maximum number of pages in the memory.
    pub maximum: Option<u64>,
    /// Whether the memory may be shared between multiple threads.
    pub shared: bool,
    /// Whether or not this is a 64-bit memory
    pub memory64: bool,
}

impl From<wasmparser::MemoryType> for Memory {
    fn from(ty: wasmparser::MemoryType) -> Memory {
        Memory {
            minimum: ty.initial,
            maximum: ty.maximum,
            shared: ty.shared,
            memory64: ty.memory64,
        }
    }
}

/// WebAssembly event.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// The event signature type.
    pub ty: TypeIndex,
}

impl From<wasmparser::TagType> for Tag {
    fn from(ty: wasmparser::TagType) -> Tag {
        match ty.kind {
            wasmparser::TagKind::Exception => Tag {
                ty: TypeIndex::from_u32(ty.func_type_idx),
            },
        }
    }
}

/// Helpers used to convert a `wasmparser` type to a type in this crate.
pub trait TypeConvert {
    /// Converts a wasmparser table type into a wasmtime type
    fn convert_global_type(&self, ty: &wasmparser::GlobalType) -> Global {
        Global {
            wasm_ty: self.convert_valtype(ty.content_type),
            mutability: ty.mutable,
        }
    }

    /// Converts a wasmparser table type into a wasmtime type
    fn convert_table_type(&self, ty: &wasmparser::TableType) -> Table {
        Table {
            wasm_ty: self.convert_ref_type(ty.element_type),
            minimum: ty.initial,
            maximum: ty.maximum,
        }
    }

    /// Converts a wasmparser function type to a wasmtime type
    fn convert_func_type(&self, ty: &wasmparser::FuncType) -> WasmFuncType {
        let params = ty
            .params()
            .iter()
            .map(|t| self.convert_valtype(*t))
            .collect();
        let results = ty
            .results()
            .iter()
            .map(|t| self.convert_valtype(*t))
            .collect();
        WasmFuncType::new(params, results)
    }

    /// Converts a wasmparser value type to a wasmtime type
    fn convert_valtype(&self, ty: wasmparser::ValType) -> WasmType {
        match ty {
            wasmparser::ValType::I32 => WasmType::I32,
            wasmparser::ValType::I64 => WasmType::I64,
            wasmparser::ValType::F32 => WasmType::F32,
            wasmparser::ValType::F64 => WasmType::F64,
            wasmparser::ValType::V128 => WasmType::V128,
            wasmparser::ValType::Ref(t) => WasmType::Ref(self.convert_ref_type(t)),
        }
    }

    /// Converts a wasmparser reference type to a wasmtime type
    fn convert_ref_type(&self, ty: wasmparser::RefType) -> WasmRefType {
        WasmRefType {
            nullable: ty.is_nullable(),
            heap_type: self.convert_heap_type(ty.heap_type()),
        }
    }

    /// Converts a wasmparser heap type to a wasmtime type
    fn convert_heap_type(&self, ty: wasmparser::HeapType) -> WasmHeapType {
        match ty {
            wasmparser::HeapType::Func => WasmHeapType::Func,
            wasmparser::HeapType::Extern => WasmHeapType::Extern,
            wasmparser::HeapType::Indexed(i) => self.lookup_heap_type(TypeIndex::from_u32(i)),

            wasmparser::HeapType::Any
            | wasmparser::HeapType::None
            | wasmparser::HeapType::NoExtern
            | wasmparser::HeapType::NoFunc
            | wasmparser::HeapType::Eq
            | wasmparser::HeapType::Struct
            | wasmparser::HeapType::Array
            | wasmparser::HeapType::I31 => {
                unimplemented!("unsupported heap type {ty:?}");
            }
        }
    }

    /// Converts the specified type index from a heap type into a canonicalized
    /// heap type.
    fn lookup_heap_type(&self, index: TypeIndex) -> WasmHeapType;
}
