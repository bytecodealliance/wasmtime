//! Internal dependency of Wasmtime and Cranelift that defines types for
//! WebAssembly.

pub use wasmparser;

use cranelift_entity::entity_impl;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

mod error;
pub use error::*;

/// A trait for things that can trace all type-to-type edges, aka all type
/// indices within this thing.
pub trait TypeTrace {
    /// Visit each edge.
    ///
    /// The function can break out of tracing by returning `Err(E)`.
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>;

    /// Visit each edge, mutably.
    ///
    /// Allows updating edges.
    ///
    /// The function can break out of tracing by returning `Err(E)`.
    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>;

    /// Canonicalize `self` by rewriting all type references inside `self` from
    /// module-level interned type indices to engine-level interned type
    /// indices.
    fn canonicalize<F>(&mut self, module_to_engine: &mut F)
    where
        F: FnMut(ModuleInternedTypeIndex) -> VMSharedTypeIndex,
    {
        self.trace_mut::<_, ()>(&mut |idx| match idx {
            EngineOrModuleTypeIndex::Engine(_) => Ok(()),
            EngineOrModuleTypeIndex::Module(module_index) => {
                let engine_index = module_to_engine(*module_index);
                *idx = EngineOrModuleTypeIndex::Engine(engine_index);
                Ok(())
            }
        })
        .unwrap()
    }
}

/// WebAssembly value type -- equivalent of `wasmparser::ValType`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmValType {
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

impl fmt::Display for WasmValType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WasmValType::I32 => write!(f, "i32"),
            WasmValType::I64 => write!(f, "i64"),
            WasmValType::F32 => write!(f, "f32"),
            WasmValType::F64 => write!(f, "f64"),
            WasmValType::V128 => write!(f, "v128"),
            WasmValType::Ref(rt) => write!(f, "{rt}"),
        }
    }
}

impl TypeTrace for WasmValType {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        match self {
            WasmValType::Ref(r) => r.trace(func),
            WasmValType::I32
            | WasmValType::I64
            | WasmValType::F32
            | WasmValType::F64
            | WasmValType::V128 => Ok(()),
        }
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        match self {
            WasmValType::Ref(r) => r.trace_mut(func),
            WasmValType::I32
            | WasmValType::I64
            | WasmValType::F32
            | WasmValType::F64
            | WasmValType::V128 => Ok(()),
        }
    }
}

impl WasmValType {
    pub fn is_vmgcref_type(&self) -> bool {
        self.is_gc_heap_type()
            || matches!(
                self,
                WasmValType::Ref(WasmRefType {
                    heap_type: WasmHeapType::I31,
                    nullable: _,
                })
            )
    }

    pub fn is_gc_heap_type(&self) -> bool {
        match self {
            WasmValType::Ref(r) => r.is_gc_heap_type(),
            _ => false,
        }
    }
}

/// WebAssembly reference type -- equivalent of `wasmparser`'s RefType
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmRefType {
    pub nullable: bool,
    pub heap_type: WasmHeapType,
}

impl TypeTrace for WasmRefType {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        self.heap_type.trace(func)
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        self.heap_type.trace_mut(func)
    }
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

    /// Is this a GC type that is allocated within the GC heap? (As opposed to
    /// `i31ref` which is a GC type that is not allocated on the GC heap.)
    pub fn is_gc_heap_type(&self) -> bool {
        self.heap_type.is_gc_heap_type()
    }
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

/// An interned type index, either at the module or engine level.
///
/// Roughly equivalent to `wasmparser::UnpackedIndex`, although doesn't have to
/// concern itself with recursion-group-local indices.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineOrModuleTypeIndex {
    /// An index within an engine, canonicalized among all modules that can
    /// interact with each other.
    Engine(VMSharedTypeIndex),
    /// An index within the current Wasm module, canonicalized within just this
    /// current module.
    Module(ModuleInternedTypeIndex),
}

impl From<ModuleInternedTypeIndex> for EngineOrModuleTypeIndex {
    fn from(i: ModuleInternedTypeIndex) -> Self {
        Self::Module(i)
    }
}

impl fmt::Display for EngineOrModuleTypeIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Engine(i) => write!(f, "(engine {})", i.bits()),
            Self::Module(i) => write!(f, "(module {})", i.as_u32()),
        }
    }
}

impl EngineOrModuleTypeIndex {
    /// Is this an engine-level type index?
    pub fn is_engine_type_index(self) -> bool {
        matches!(self, Self::Engine(_))
    }

    /// Get the underlying engine-level type index, if any.
    pub fn as_engine_type_index(self) -> Option<VMSharedTypeIndex> {
        match self {
            Self::Engine(e) => Some(e),
            Self::Module(_) => None,
        }
    }

    /// Get the underlying engine-level type index, or panic.
    pub fn unwrap_engine_type_index(self) -> VMSharedTypeIndex {
        self.as_engine_type_index()
            .expect("`unwrap_engine_type_index` on module type index")
    }

    /// Is this an module-level type index?
    pub fn is_module_type_index(self) -> bool {
        matches!(self, Self::Module(_))
    }

    /// Get the underlying module-level type index, if any.
    pub fn as_module_type_index(self) -> Option<ModuleInternedTypeIndex> {
        match self {
            Self::Module(e) => Some(e),
            Self::Engine(_) => None,
        }
    }

    /// Get the underlying module-level type index, or panic.
    pub fn unwrap_module_type_index(self) -> ModuleInternedTypeIndex {
        self.as_module_type_index()
            .expect("`unwrap_module_type_index` on engine type index")
    }
}

/// WebAssembly heap type -- equivalent of `wasmparser`'s HeapType
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmHeapType {
    Extern,
    Func,
    Concrete(EngineOrModuleTypeIndex),
    NoFunc,
    Any,
    I31,
    None,
}

impl fmt::Display for WasmHeapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Extern => write!(f, "extern"),
            Self::Func => write!(f, "func"),
            Self::Concrete(i) => write!(f, "{i}"),
            Self::NoFunc => write!(f, "nofunc"),
            Self::Any => write!(f, "any"),
            Self::I31 => write!(f, "i31"),
            Self::None => write!(f, "none"),
        }
    }
}

impl TypeTrace for WasmHeapType {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        match *self {
            Self::Concrete(i) => func(i),
            _ => Ok(()),
        }
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        match self {
            Self::Concrete(i) => func(i),
            _ => Ok(()),
        }
    }
}

impl WasmHeapType {
    /// Is this a GC type that is allocated within the GC heap? (As opposed to
    /// `i31ref` which is a GC type that is not allocated on the GC heap.)
    pub fn is_gc_heap_type(&self) -> bool {
        // All `t <: (ref null any)` and `t <: (ref null extern)` that are
        // not `(ref null? i31)` are GC-managed references.
        match self {
            // These types are managed by the GC.
            Self::Extern | Self::Any => true,

            // TODO: Once we support concrete struct and array types, we will
            // need to look at the payload to determine whether the type is
            // GC-managed or not.
            Self::Concrete(_) => false,

            // These are compatible with GC references, but don't actually point
            // to GC objects.
            Self::I31 => false,

            // These are a subtype of GC-managed types, but are uninhabited, so
            // can never actually point to a GC object. Again, we could return
            // `true` here but there is no need.
            Self::None => false,

            // These types are not managed by the GC.
            Self::Func | Self::NoFunc => false,
        }
    }
}

/// WebAssembly function type -- equivalent of `wasmparser`'s FuncType.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WasmFuncType {
    params: Box<[WasmValType]>,
    non_i31_gc_ref_params_count: usize,
    returns: Box<[WasmValType]>,
    non_i31_gc_ref_returns_count: usize,
}

impl TypeTrace for WasmFuncType {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        for p in self.params.iter() {
            p.trace(func)?;
        }
        for r in self.returns.iter() {
            r.trace(func)?;
        }
        Ok(())
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        for p in self.params.iter_mut() {
            p.trace_mut(func)?;
        }
        for r in self.returns.iter_mut() {
            r.trace_mut(func)?;
        }
        Ok(())
    }
}

impl WasmFuncType {
    #[inline]
    pub fn new(params: Box<[WasmValType]>, returns: Box<[WasmValType]>) -> Self {
        let non_i31_gc_ref_params_count = params
            .iter()
            .filter(|p| match **p {
                WasmValType::Ref(rt) => {
                    rt.heap_type != WasmHeapType::I31 && rt.heap_type != WasmHeapType::Func
                }
                _ => false,
            })
            .count();
        let non_i31_gc_ref_returns_count = returns
            .iter()
            .filter(|r| match **r {
                WasmValType::Ref(rt) => {
                    rt.heap_type != WasmHeapType::I31 && rt.heap_type != WasmHeapType::Func
                }
                _ => false,
            })
            .count();
        WasmFuncType {
            params,
            non_i31_gc_ref_params_count,
            returns,
            non_i31_gc_ref_returns_count,
        }
    }

    /// Function params types.
    #[inline]
    pub fn params(&self) -> &[WasmValType] {
        &self.params
    }

    /// How many `externref`s are in this function's params?
    #[inline]
    pub fn non_i31_gc_ref_params_count(&self) -> usize {
        self.non_i31_gc_ref_params_count
    }

    /// Returns params types.
    #[inline]
    pub fn returns(&self) -> &[WasmValType] {
        &self.returns
    }

    /// How many `externref`s are in this function's returns?
    #[inline]
    pub fn non_i31_gc_ref_returns_count(&self) -> usize {
        self.non_i31_gc_ref_returns_count
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

/// Index type of a type (imported or defined) inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct TypeIndex(u32);
entity_impl!(TypeIndex);

/// A canonicalized type index for a type within a single WebAssembly module.
///
/// Note that this is deduplicated only at the level of a single WebAssembly
/// module, not at the level of a whole store or engine. This means that these
/// indices are only unique within the context of a single Wasm module, and
/// therefore are not suitable for runtime type checks (which, in general, may
/// involve entities defined in different modules).
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct ModuleInternedTypeIndex(u32);
entity_impl!(ModuleInternedTypeIndex);

/// A canonicalized type index into an engine's shared type registry.
///
/// This is canonicalized/deduped at the level of a whole engine, across all the
/// modules loaded into that engine, not just at the level of a single
/// particular module. This means that `VMSharedTypeIndex` is usable for
/// e.g. checking that function signatures match during an indirect call
/// (potentially to a function defined in a different module) at runtime.
#[repr(transparent)] // Used directly by JIT code.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct VMSharedTypeIndex(u32);
entity_impl!(VMSharedTypeIndex);

impl VMSharedTypeIndex {
    /// Create a new `VMSharedTypeIndex`.
    #[inline]
    pub fn new(value: u32) -> Self {
        assert_ne!(
            value,
            u32::MAX,
            "u32::MAX is reserved for the default value"
        );
        Self(value)
    }

    /// Returns the underlying bits of the index.
    #[inline]
    pub fn bits(&self) -> u32 {
        self.0
    }
}

impl Default for VMSharedTypeIndex {
    #[inline]
    fn default() -> Self {
        Self(u32::MAX)
    }
}

/// Index type of a passive data segment inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct DataIndex(u32);
entity_impl!(DataIndex);

/// Index type of a passive element segment inside the WebAssembly module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct ElemIndex(u32);
entity_impl!(ElemIndex);

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
    Function(EngineOrModuleTypeIndex),
}

impl TypeTrace for EntityType {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        match self {
            Self::Global(g) => g.trace(func),
            Self::Table(t) => t.trace(func),
            Self::Function(idx) => func(*idx),
            Self::Memory(_) | Self::Tag(_) => Ok(()),
        }
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        match self {
            Self::Global(g) => g.trace_mut(func),
            Self::Table(t) => t.trace_mut(func),
            Self::Function(idx) => func(idx),
            Self::Memory(_) | Self::Tag(_) => Ok(()),
        }
    }
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
    pub fn unwrap_func(&self) -> EngineOrModuleTypeIndex {
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
    pub wasm_ty: crate::WasmValType,
    /// A flag indicating whether the value may change at runtime.
    pub mutability: bool,
}

impl TypeTrace for Global {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        let Global {
            wasm_ty,
            mutability: _,
        } = self;
        wasm_ty.trace(func)
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        let Global {
            wasm_ty,
            mutability: _,
        } = self;
        wasm_ty.trace_mut(func)
    }
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
    /// A `(ref.i31 (global.get N))` initializer.
    RefI31Const(i32),
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

impl TypeTrace for Table {
    fn trace<F, E>(&self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        let Table {
            wasm_ty,
            minimum: _,
            maximum: _,
        } = self;
        wasm_ty.trace(func)
    }

    fn trace_mut<F, E>(&mut self, func: &mut F) -> Result<(), E>
    where
        F: FnMut(&mut EngineOrModuleTypeIndex) -> Result<(), E>,
    {
        let Table {
            wasm_ty,
            minimum: _,
            maximum: _,
        } = self;
        wasm_ty.trace_mut(func)
    }
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
    fn convert_valtype(&self, ty: wasmparser::ValType) -> WasmValType {
        match ty {
            wasmparser::ValType::I32 => WasmValType::I32,
            wasmparser::ValType::I64 => WasmValType::I64,
            wasmparser::ValType::F32 => WasmValType::F32,
            wasmparser::ValType::F64 => WasmValType::F64,
            wasmparser::ValType::V128 => WasmValType::V128,
            wasmparser::ValType::Ref(t) => WasmValType::Ref(self.convert_ref_type(t)),
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
            wasmparser::HeapType::Extern => WasmHeapType::Extern,
            wasmparser::HeapType::Func => WasmHeapType::Func,
            wasmparser::HeapType::NoFunc => WasmHeapType::NoFunc,
            wasmparser::HeapType::Concrete(i) => self.lookup_heap_type(i),
            wasmparser::HeapType::Any => WasmHeapType::Any,
            wasmparser::HeapType::I31 => WasmHeapType::I31,
            wasmparser::HeapType::None => WasmHeapType::None,

            wasmparser::HeapType::Exn
            | wasmparser::HeapType::NoExn
            | wasmparser::HeapType::NoExtern
            | wasmparser::HeapType::Eq
            | wasmparser::HeapType::Struct
            | wasmparser::HeapType::Array => {
                unimplemented!("unsupported heap type {ty:?}");
            }
        }
    }

    /// Converts the specified type index from a heap type into a canonicalized
    /// heap type.
    fn lookup_heap_type(&self, index: wasmparser::UnpackedIndex) -> WasmHeapType;
}
