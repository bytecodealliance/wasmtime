use crate::translate::Load;
use core::fmt;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{self, InstBuilder as _},
};
use wasmtime_environ::{
    BuiltinFunctionIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, GetPtrSize,
    ModuleInternedTypeIndex, NUM_COMPONENT_CONTEXT_SLOTS, PtrSize as _, RuntimeDataIndex,
    StaticModuleIndex, VMOffsets, VmctxArrayIndex as _,
    component::{
        ComponentBuiltinFunctionIndex, LoweredIndex, ResourceIndex, RuntimeCallbackIndex,
        RuntimeComponentInstanceIndex, RuntimeMemoryIndex, RuntimePostReturnIndex,
        RuntimeReallocIndex, VMComponentOffsets,
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum VmType {
    VMContext,
    VMStoreContext,
    VMMemoryDefinition,
    VMTableDefinition,
    // NB: these two are currently only referenced by macro-generated
    // `AliasRegions` helpers that are not all wired up to call sites yet.
    #[allow(
        dead_code,
        reason = "generated uniformly for all VM types via `for_each_vm_type!`"
    )]
    VMGlobalDefinition,
    #[allow(
        dead_code,
        reason = "generated uniformly for all VM types via `for_each_vm_type!`"
    )]
    VMTagDefinition,
    VMComponentContext,
    VMDrcHeapData,
    VMCopyingHeapData,
    VMNullHeapData,
    VMDeferredThread,
    #[allow(
        dead_code,
        reason = "generated uniformly for all VM types via `for_each_vm_type!`"
    )]
    VMLazyThread,
    VMContRef,
    ContinuationStackMemory,
    VMFunctionImport,
    VMMemoryImport,
    VMTableImport,
    VMTagImport,
    VMGlobalImport,
    VMFuncRef,
    TypeIdsArray,
    EpochCounter,
    BuiltinFunctionsArray,
    ComponentBuiltinFunctionsArray,
    HostValRaw,
}

/// A key that uniquely identifies an alias region across an entire compilation.
///
/// This is used to assign stable `user_id`s to `AliasRegionData` entries so
/// that alias regions can be deduplicated during inlining.
///
/// The key encodes into a single `u32` with the following layout:
/// `[ kind: 6 bits | data: 26 bits ]`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum AliasRegionKey {
    /// An access of a field within a VM data structure of type `ty`.
    Vm {
        /// The type of VM data structure being accessed.
        ty: VmType,
        /// The offset of the accessed field *within* the `ty` structure (or
        /// the base offset of the array, for array fields).
        offset: u32,
    },

    /// An access of a memory that crosses a module boundary and whose
    /// definition we do not statically know (shared across all such memories).
    PublicMemory,

    /// A defined memory access.
    DefinedMemory {
        /// The static module index.
        module: StaticModuleIndex,
        /// The defined memory index within the module.
        index: DefinedMemoryIndex,
    },

    /// An access of a table that crosses a module boundary and whose definition
    /// we do not statically know (shared across all such tables).
    PublicTable,

    /// A defined table access.
    DefinedTable {
        /// The static module index.
        module: StaticModuleIndex,
        /// The defined table index within the module.
        index: DefinedTableIndex,
    },

    /// An access of a global that crosses a module boundary and whose definition
    /// we do not statically know (shared across all such globals).
    PublicGlobal,

    /// A defined global access.
    DefinedGlobal {
        /// The static module index.
        module: StaticModuleIndex,
        /// The defined global index within the module.
        index: DefinedGlobalIndex,
    },

    /// A GC heap access.
    GcHeap,

    /// A stack slot access.
    Stack {
        /// The stack slot being accessed.
        slot: ir::StackSlot,
    },

    /// All unsafe intrinsics share a single alias region.
    ///
    /// By contract, unsafe intrinsics cannot access our internal `VM*` types or
    /// linear memories or anything else that has its own dedicated alias
    /// region. However, we cannot guarantee anything about the (lack of)
    /// aliasing of the embedder's data structures that the unsafe intrinsics do
    /// access, so all their accesses are lumped together into the same region.
    UnsafeIntrinsicMemory,

    /// An access of a `ValRaw` inside a passive element segment.
    ElementSegment,

    /// An access of the bytes inside a data segment.
    DataSegment,
}

impl AliasRegionKey {
    const KIND_BITS: u32 = 6;
    const KIND_OFFSET: u32 = 32 - Self::KIND_BITS;
    const KIND_MASK: u32 = ((1 << Self::KIND_BITS) - 1) << Self::KIND_OFFSET;

    const OFFSET_MASK: u32 = !Self::KIND_MASK;

    const MODULE_BITS: u32 = 8;
    const MODULE_OFFSET: u32 = Self::KIND_OFFSET - Self::MODULE_BITS;
    const MODULE_MASK: u32 = ((1 << Self::MODULE_BITS) - 1) << Self::MODULE_OFFSET;

    const INDEX_MASK: u32 = !Self::KIND_MASK & !Self::MODULE_MASK;

    const fn new_kind(kind: u32) -> u32 {
        assert!(kind < (1 << Self::KIND_BITS));
        kind << Self::KIND_OFFSET
    }

    const VM_CONTEXT_KIND: u32 = Self::new_kind(0b000000);
    const VM_STORE_CONTEXT_KIND: u32 = Self::new_kind(0b000001);
    const IMPORTED_MEMORY_KIND: u32 = Self::new_kind(0b000010);
    const DEFINED_MEMORY_KIND: u32 = Self::new_kind(0b000011);
    const IMPORTED_TABLE_KIND: u32 = Self::new_kind(0b000100);
    const DEFINED_TABLE_KIND: u32 = Self::new_kind(0b000101);
    const IMPORTED_GLOBAL_KIND: u32 = Self::new_kind(0b000110);
    const DEFINED_GLOBAL_KIND: u32 = Self::new_kind(0b000111);
    const GC_HEAP_KIND: u32 = Self::new_kind(0b001000);
    const VM_MEMORY_DEFINITION_KIND: u32 = Self::new_kind(0b001001);
    const VM_TABLE_DEFINITION_KIND: u32 = Self::new_kind(0b001010);
    const VM_COMPONENT_CONTEXT_KIND: u32 = Self::new_kind(0b001011);
    const VM_DRC_HEAP_DATA_KIND: u32 = Self::new_kind(0b001100);
    const VM_COPYING_HEAP_DATA_KIND: u32 = Self::new_kind(0b001101);
    const VM_NULL_HEAP_DATA_KIND: u32 = Self::new_kind(0b001110);
    const VM_DEFERRED_THREAD_KIND: u32 = Self::new_kind(0b001111);
    const VM_CONTREF_KIND: u32 = Self::new_kind(0b010000);
    const CONTINUATION_STACK_MEMORY_KIND: u32 = Self::new_kind(0b010001);
    const VM_FUNCTION_IMPORT_KIND: u32 = Self::new_kind(0b010010);
    const VM_MEMORY_IMPORT_KIND: u32 = Self::new_kind(0b010011);
    const VM_TABLE_IMPORT_KIND: u32 = Self::new_kind(0b010100);
    const VM_TAG_IMPORT_KIND: u32 = Self::new_kind(0b010101);
    const VM_GLOBAL_IMPORT_KIND: u32 = Self::new_kind(0b010110);
    const STACK_KIND: u32 = Self::new_kind(0b010111);
    const VM_FUNC_REF_KIND: u32 = Self::new_kind(0b011000);
    const TYPE_IDS_ARRAY_KIND: u32 = Self::new_kind(0b011001);
    const EPOCH_COUNTER_KIND: u32 = Self::new_kind(0b011010);
    const BUILTIN_FUNCTIONS_KIND: u32 = Self::new_kind(0b011011);
    const COMPONENT_BUILTIN_FUNCTIONS_KIND: u32 = Self::new_kind(0b011100);
    const UNSAFE_INTRINSIC_MEMORY_KIND: u32 = Self::new_kind(0b011101);
    const HOST_VAL_RAW_KIND: u32 = Self::new_kind(0b011110);
    const ELEMENT_SEGMENT_KIND: u32 = Self::new_kind(0b011111);
    const DATA_SEGMENT_KIND: u32 = Self::new_kind(0b100000);
    const VM_GLOBAL_DEFINITION_KIND: u32 = Self::new_kind(0b100001);
    const VM_TAG_DEFINITION_KIND: u32 = Self::new_kind(0b100010);
    const VM_LAZY_THREAD_KIND: u32 = Self::new_kind(0b100011);

    /// Encode this key into a raw `u32` suitable for use as an
    /// `AliasRegionData::user_id`.
    pub(crate) fn into_raw(self) -> u32 {
        match self {
            AliasRegionKey::Vm { ty, offset } => {
                debug_assert_eq!(offset & Self::KIND_MASK, 0);
                let kind = match ty {
                    VmType::VMContext => Self::VM_CONTEXT_KIND,
                    VmType::VMStoreContext => Self::VM_STORE_CONTEXT_KIND,
                    VmType::VMMemoryDefinition => Self::VM_MEMORY_DEFINITION_KIND,
                    VmType::VMTableDefinition => Self::VM_TABLE_DEFINITION_KIND,
                    VmType::VMGlobalDefinition => Self::VM_GLOBAL_DEFINITION_KIND,
                    VmType::VMTagDefinition => Self::VM_TAG_DEFINITION_KIND,
                    VmType::VMComponentContext => Self::VM_COMPONENT_CONTEXT_KIND,
                    VmType::VMDrcHeapData => Self::VM_DRC_HEAP_DATA_KIND,
                    VmType::VMCopyingHeapData => Self::VM_COPYING_HEAP_DATA_KIND,
                    VmType::VMNullHeapData => Self::VM_NULL_HEAP_DATA_KIND,
                    VmType::VMDeferredThread => Self::VM_DEFERRED_THREAD_KIND,
                    VmType::VMLazyThread => Self::VM_LAZY_THREAD_KIND,
                    VmType::VMContRef => Self::VM_CONTREF_KIND,
                    VmType::ContinuationStackMemory => Self::CONTINUATION_STACK_MEMORY_KIND,
                    VmType::VMFunctionImport => Self::VM_FUNCTION_IMPORT_KIND,
                    VmType::VMMemoryImport => Self::VM_MEMORY_IMPORT_KIND,
                    VmType::VMTableImport => Self::VM_TABLE_IMPORT_KIND,
                    VmType::VMTagImport => Self::VM_TAG_IMPORT_KIND,
                    VmType::VMGlobalImport => Self::VM_GLOBAL_IMPORT_KIND,
                    VmType::VMFuncRef => Self::VM_FUNC_REF_KIND,
                    VmType::TypeIdsArray => Self::TYPE_IDS_ARRAY_KIND,
                    VmType::EpochCounter => Self::EPOCH_COUNTER_KIND,
                    VmType::BuiltinFunctionsArray => Self::BUILTIN_FUNCTIONS_KIND,
                    VmType::ComponentBuiltinFunctionsArray => {
                        Self::COMPONENT_BUILTIN_FUNCTIONS_KIND
                    }
                    VmType::HostValRaw => Self::HOST_VAL_RAW_KIND,
                };
                kind | (offset & Self::OFFSET_MASK)
            }
            AliasRegionKey::PublicMemory => Self::IMPORTED_MEMORY_KIND,
            AliasRegionKey::DefinedMemory { module, index } => {
                debug_assert_eq!(
                    module.as_u32() & !(Self::MODULE_MASK >> Self::MODULE_OFFSET),
                    0
                );
                debug_assert_eq!(index.as_u32() & !Self::INDEX_MASK, 0);
                Self::DEFINED_MEMORY_KIND
                    | (module.as_u32() << Self::MODULE_OFFSET)
                    | index.as_u32()
            }
            AliasRegionKey::PublicTable => Self::IMPORTED_TABLE_KIND,
            AliasRegionKey::DefinedTable { module, index } => {
                debug_assert_eq!(
                    module.as_u32() & !(Self::MODULE_MASK >> Self::MODULE_OFFSET),
                    0
                );
                debug_assert_eq!(index.as_u32() & !Self::INDEX_MASK, 0);
                Self::DEFINED_TABLE_KIND | (module.as_u32() << Self::MODULE_OFFSET) | index.as_u32()
            }
            AliasRegionKey::PublicGlobal => Self::IMPORTED_GLOBAL_KIND,
            AliasRegionKey::DefinedGlobal { module, index } => {
                debug_assert_eq!(
                    module.as_u32() & !(Self::MODULE_MASK >> Self::MODULE_OFFSET),
                    0
                );
                debug_assert_eq!(index.as_u32() & !Self::INDEX_MASK, 0);
                Self::DEFINED_GLOBAL_KIND
                    | (module.as_u32() << Self::MODULE_OFFSET)
                    | index.as_u32()
            }
            AliasRegionKey::GcHeap => Self::GC_HEAP_KIND,
            AliasRegionKey::Stack { slot } => {
                debug_assert_eq!(slot.as_u32() & Self::KIND_MASK, 0);
                Self::STACK_KIND | (slot.as_u32() & Self::OFFSET_MASK)
            }
            AliasRegionKey::UnsafeIntrinsicMemory => Self::UNSAFE_INTRINSIC_MEMORY_KIND,
            AliasRegionKey::ElementSegment => Self::ELEMENT_SEGMENT_KIND,
            AliasRegionKey::DataSegment => Self::DATA_SEGMENT_KIND,
        }
    }
}

impl fmt::Debug for AliasRegionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AliasRegionKey::Vm { ty, offset } => write!(f, "{ty:?}+{offset:#x}"),
            AliasRegionKey::PublicMemory => write!(f, "PublicMemory"),
            AliasRegionKey::DefinedMemory { module, index } => {
                write!(f, "DefinedMemory({module:?}, {index:?})")
            }
            AliasRegionKey::PublicTable => write!(f, "PublicTable"),
            AliasRegionKey::DefinedTable { module, index } => {
                write!(f, "DefinedTable({module:?}, {index:?})")
            }
            AliasRegionKey::PublicGlobal => write!(f, "PublicGlobal"),
            AliasRegionKey::DefinedGlobal { module, index } => {
                write!(f, "DefinedGlobal({module:?}, {index:?})")
            }
            AliasRegionKey::GcHeap => write!(f, "GcHeap"),
            AliasRegionKey::Stack { slot } => write!(f, "Stack({slot:?})"),
            AliasRegionKey::UnsafeIntrinsicMemory => write!(f, "UnsafeIntrinsicMemory"),
            AliasRegionKey::ElementSegment => write!(f, "ElementSegment"),
            AliasRegionKey::DataSegment => write!(f, "DataSegment"),
        }
    }
}

impl From<AliasRegionKey> for ir::AliasRegionData {
    fn from(key: AliasRegionKey) -> ir::AliasRegionData {
        ir::AliasRegionData {
            user_id: key.into_raw(),
            description: format!("{key:?}").into(),
        }
    }
}

/// Alias region cache and load/store helper type.
pub struct AliasRegions<Offsets> {
    pointer_type: ir::Type,
    offsets: Offsets,

    /// Cached alias regions for alias analysis.
    ///
    /// Avoids allocating a string for the debug formatting of `AliasRegionKey`
    /// as the `ir::AliasRegionData::description` string repeatedly.
    cache: std::collections::HashMap<AliasRegionKey, ir::AliasRegion>,
}

impl<Offsets> AliasRegions<Offsets> {
    /// Get the offsets this `AliasRegions` computes its field offsets from.
    pub fn offsets(&self) -> &Offsets {
        &self.offsets
    }

    /// Make the alias region for a stack map.
    pub fn stack_map_region(
        regions: &mut ir::AliasRegionSet,
        _ty: ir::Type,
        slot: ir::StackSlot,
        _offset: u32,
    ) -> Option<ir::AliasRegion> {
        let key = AliasRegionKey::Stack { slot };
        let id = key.into_raw();
        if let Some(region) = regions.get(id) {
            Some(region)
        } else {
            Some(regions.insert(key.into()))
        }
    }

    /// Get the alias region for a stack slot.
    pub fn stack_slot_region(
        &mut self,
        func: &mut ir::Function,
        slot: ir::StackSlot,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::Stack { slot })
    }

    /// Get the alias region for the given key.
    fn region(&mut self, func: &mut ir::Function, key: AliasRegionKey) -> ir::AliasRegion {
        *self
            .cache
            .entry(key)
            .or_insert_with(|| func.dfg.alias_regions.insert(key.into()))
    }
}

/// A single field within one of Wasmtime's `VM*` types, along with everything
/// needed to emit a correctly alias-analyzed load or store of that field.
///
/// A `Field` is obtained from the `Field`-returning accessors generated by
/// [`define_vm_type_alias_region_helpers!`], for example
/// `alias_regions.vm_memory_definition().base()`.
///
/// This is the evolved form of the old `Load` type: in addition to producing a
/// deferred [`Load`] descriptor (via [`Field::to_deferred_load`]) for use in a
/// `VmctxLoadChain`, a `Field` can directly emit loads and stores relative to a
/// pointer to the containing `VM*` structure.
pub struct Field<'a, Offsets> {
    /// The alias-region cache used to get-or-create this field's alias region.
    regions: &'a mut AliasRegions<Offsets>,
    /// The key identifying this field's alias region.
    ///
    /// This is fixed when the `Field` is created, derived from the field's
    /// offset *within* its containing `VM*` type, and is deliberately
    /// independent of [`Field::relative_to`]: rebasing the load/store
    /// (e.g. to make the access relative to the `vmctx` rather than to a pointer
    /// to the containing structure) must not change which alias region the
    /// access belongs to.
    key: AliasRegionKey,
    /// The offset added to the base value when emitting a load or store of this
    /// field.
    ///
    /// Initially the field's offset *within* its containing `VM*` type; callers
    /// may rebase it via [`Field::relative_to`].
    offset: u32,
    /// The base memory flags for accesses of this field, before this field's
    /// alias region is mixed in.
    flags: ir::MemFlagsData,
    /// The Cranelift type of this field.
    ty: ir::Type,
}

impl<'a, Offsets> Field<'a, Offsets> {
    /// Create a new `Field` for the given `offset` within `vm_type`, loaded or
    /// stored with the given base `flags` and Cranelift type `ty`.
    fn new(
        regions: &'a mut AliasRegions<Offsets>,
        vm_type: VmType,
        offset: u32,
        flags: ir::MemFlagsData,
        ty: ir::Type,
    ) -> Self {
        Field {
            regions,
            key: AliasRegionKey::Vm {
                ty: vm_type,
                offset,
            },
            offset,
            flags,
            ty,
        }
    }

    /// Mark accesses of this field as `readonly`.
    ///
    /// Whether a field is `readonly` often depends on dynamic properties of the
    /// module being compiled (e.g. whether a memory can be relocated) rather
    /// than being a static property of the field; this method allows callers
    /// to mark the load as `readonly` in these cases.
    pub fn readonly(mut self) -> Self {
        self.flags.set_readonly();
        self
    }

    /// Mark accesses of this field as `readonly` if and only if `readonly` is
    /// `true`.
    ///
    /// See the note on [`Field::readonly`].
    pub fn readonly_if(self, readonly: bool) -> Self {
        if readonly { self.readonly() } else { self }
    }

    /// Mark accesses of this field as `can_move`.
    ///
    /// See the note on [`Field::readonly`].
    pub fn can_move(mut self) -> Self {
        self.flags = self.flags.with_can_move();
        self
    }

    /// Mark accesses of this field as `can_move` if and only if `can_move` is
    /// `true`.
    ///
    /// See the note on [`Field::readonly`].
    pub fn can_move_if(self, can_move: bool) -> Self {
        if can_move { self.can_move() } else { self }
    }

    /// Set the trap code for accesses of this field.
    ///
    /// A `Field`'s accesses do not trap by default.
    ///
    /// Note that when signals-based traps are disabled, callers must use the
    /// explicit call-to-host trapping codegen instead.
    pub fn trap_code(mut self, code: Option<ir::TrapCode>) -> Self {
        self.flags = self.flags.with_trap_code(code);
        self
    }

    /// Cast this field to the given type.
    ///
    /// This can be used, for example, to cast a `Field` that points to a
    /// `VMGlobalDefinition`'s storage (a `[u8; 16]` represented as
    /// `ir::types::I8X16`) to the global's actual Wasm type's representation
    /// (`ir::types::I32` for a Wasm `i32`).
    pub fn cast(mut self, ty: ir::Type) -> Self {
        self.ty = ty;
        self
    }

    /// Rebase this field's load or store to be relative to a new base.
    ///
    /// The `struct_offset` parameter is the offset of this field's containing
    /// `VM*` structure within the new base.
    ///
    /// A `Field` starts out relative to a pointer to its containing `VM*`
    /// structure. When that structure is inlined directly into a larger one (as
    /// an owned memory's `VMMemoryDefinition` is inlined into the `vmctx`), use
    /// this method to fold the structure's own offset within the larger
    /// structure into this `Field`, so that the resulting access is relative to
    /// the larger structure directly.
    pub fn relative_to(mut self, struct_offset: u32) -> Self {
        self.offset += struct_offset;
        self
    }

    /// Get-or-create this field's alias region.
    pub fn region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.regions.region(func, self.key)
    }

    /// Get-or-create this field's alias region and mix it into this field's
    /// base flags.
    fn flags_with_region(&mut self, func: &mut ir::Function) -> ir::MemFlagsData {
        let region = self.region(func);
        self.flags.with_alias_region(Some(region))
    }

    /// Get a deferred [`Load`] descriptor for this field, for use in a
    /// `VmctxLoadChain`.
    ///
    /// The load is emitted at the field's current offset relative to its base
    /// value. By default that is the field's offset within its containing `VM*`
    /// structure, so the load is relative to a pointer to that structure; to
    /// make it relative to something else (e.g. the `vmctx`, when the structure
    /// is inlined into the `vmctx`), call [`Field::relative_to`] first.
    pub fn to_deferred_load(&mut self, func: &mut ir::Function) -> Load {
        let flags = self.flags_with_region(func);
        Load {
            offset: self.offset,
            flags,
            ty: self.ty,
        }
    }

    /// Emit a load of this field relative to `ptr`, a pointer to the containing
    /// `VM*` structure.
    pub fn load(&mut self, cursor: &mut FuncCursor<'_>, ptr: ir::Value) -> ir::Value {
        let load = self.to_deferred_load(cursor.func);
        load.emit(cursor, ptr)
    }

    /// Emit an atomic load of this field relative to `ptr`, a pointer to the
    /// containing `VM*` structure.
    pub fn load_atomic(&mut self, cursor: &mut FuncCursor<'_>, ptr: ir::Value) -> ir::Value {
        let flags = self.flags_with_region(cursor.func);
        let ty = self.ty;
        let pointer_type = self.regions.pointer_type;
        let offset = cursor.ins().iconst(pointer_type, i64::from(self.offset));
        let addr = cursor.ins().iadd(ptr, offset);
        cursor.ins().atomic_load(ty, flags, addr)
    }

    /// Emit a store of `value` to this field relative to `ptr`, a pointer to
    /// the containing `VM*` structure.
    pub fn store(&mut self, cursor: &mut FuncCursor<'_>, ptr: ir::Value, value: ir::Value) {
        let flags = self.flags_with_region(cursor.func);
        cursor
            .ins()
            .store(flags, value, ptr, i32::try_from(self.offset).unwrap());
    }
}

/// Define, for each `VM*` type, an `AliasRegions` accessor that returns a
/// wrapper exposing a [`Field`]-returning method per field of that type.
///
/// For example, given
///
/// ```ignore
/// struct VMMemoryDefinition {
///     base: VmPtr<u8>,
///     current_length: AtomicUsize,
/// }
/// ```
///
/// this macro generates:
///
/// ```ignore
/// impl<Offsets> AliasRegions<Offsets> {
///     fn vm_memory_definition(&mut self) -> VMMemoryDefinition<'_, Offsets> { ... }
/// }
///
/// struct VMMemoryDefinition<'a, Offsets> { ... }
///
/// impl<'a, Offsets: GetPtrSize> VMMemoryDefinition<'a, Offsets> {
///     fn base(self) -> Field<'a, Offsets> { ... }
///     fn current_length(self) -> Field<'a, Offsets> { ... }
/// }
/// ```
///
/// A field marked `#[aggregate]` gets no accessor, because it has no single
/// Cranelift type. A field marked `#[indexed]` is a fixed-size array of a
/// scalar type; its accessor takes the element index, and each element gets its
/// own alias region.
#[allow(
    unused_macro_rules,
    reason = "the `#[readonly]`/`#[can_move]` marker arms are generated \
              uniformly but not exercised until a VM type uses those markers"
)]
macro_rules! define_vm_type_alias_region_helpers {
    // `UnsafeCell<T>` is `repr(transparent)` and is accessed exactly as its `T`
    // would be; delegate to the inner type.
    (@field_ty $pt:expr, UnsafeCell < $inner:tt >) => {
        define_vm_type_alias_region_helpers!(@field_ty $pt, $inner)
    };

    // Classify a field type to its Cranelift `ir::Type`, given `$pt` (the
    // target pointer type as an `ir::Type`).
    //
    // Note that there is deliberately no arm for a composite field (a nested
    // struct, an array, or a range): such a field has no single Cranelift type,
    // and is marked `#[aggregate]` in `for_each_vm_type!` so that no accessor is
    // generated for it at all.
    (@field_ty $pt:expr, VmPtr < $g:ty >) => { $pt };
    (@field_ty $pt:expr, Option < VmPtr < $g:ty >>) => { $pt };
    (@field_ty $pt:expr, AtomicUsize) => { $pt };
    (@field_ty $pt:expr, usize) => { $pt };
    (@field_ty $pt:expr, i64) => { ir::types::I64 };
    (@field_ty $pt:expr, u64) => { ir::types::I64 };
    (@field_ty $pt:expr, u32) => { ir::types::I32 };
    (@field_ty $pt:expr, NonZeroU32) => { ir::types::I32 };
    (@field_ty $pt:expr, Option < VMGcRef >) => { ir::types::I32 };
    // `VMLazyThread` is a pointer-sized bitpacked integer; see its definition in
    // `for_each_vm_type!`.
    (@field_ty $pt:expr, VMLazyThread) => { $pt };
    (@field_ty $pt:expr, [u8; 16]) => { ir::types::I8X16 };
    (@field_ty $pt:expr, VMSharedTypeIndex) => { ir::types::I32 };
    (@field_ty $pt:expr, DefinedTableIndex) => { ir::types::I32 };
    (@field_ty $pt:expr, DefinedMemoryIndex) => { ir::types::I32 };
    (@field_ty $pt:expr, DefinedTagIndex) => { ir::types::I32 };
    (@field_ty $pt:expr, VMGlobalKind) => { ir::types::I64 };

    // Classify an `#[indexed]` field's array type to
    // `(element ir::Type, element size in bytes, element count)`.
    (@array_info $pt:expr, UnsafeCell < $inner:tt >) => {
        define_vm_type_alias_region_helpers!(@array_info $pt, $inner)
    };
    (@array_info $pt:expr, [u32; $n:expr]) => { (ir::types::I32, 4u32, $n) };

    // Apply a field attribute to its access flags. The `#[readonly]` and
    // `#[can_move]` markers map to the corresponding `MemFlagsData` builder
    // methods; doc comments have no effect on flags; any other attribute is a
    // compile error.
    (@apply_attr $flags:expr, [readonly]) => { $flags.with_readonly() };
    (@apply_attr $flags:expr, [can_move]) => { $flags.with_can_move() };
    (@apply_attr $flags:expr, [doc = $d:literal]) => { $flags };

    // Emit the accessor method for a single ordinary (scalar) field.
    (@method $Name:ident $snake:ident plain $fname:ident [ $($fattr:tt)* ] [ $($fty:tt)* ]) => {
        #[doc = concat!(
            "Get the [`Field`] for the `", stringify!($fname),
            "` field of `", stringify!($Name), "`."
        )]
        pub fn $fname(self) -> Field<'a, Offsets> {
            let offset = u32::from(
                self.regions.offsets.get_ptr_size().$snake().$fname()
            );
            let flags = ir::MemFlagsData::trusted();
            $(
                let flags = define_vm_type_alias_region_helpers!(
                    @apply_attr flags, $fattr
                );
            )*
            let ty = define_vm_type_alias_region_helpers!(
                @field_ty self.regions.pointer_type, $($fty)*
            );
            Field::new(self.regions, VmType::$Name, offset, flags, ty)
        }
    };

    // Emit the accessor method for an `#[indexed]` field: a fixed-size array of
    // a scalar type, indexed by a compile-time constant.
    (@method $Name:ident $snake:ident indexed $fname:ident [ $($fattr:tt)* ] [ $($fty:tt)* ]) => {
        #[doc = concat!(
            "Get the [`Field`] for the `index`th element of the `",
            stringify!($fname), "` field of `", stringify!($Name), "`."
        )]
        pub fn $fname(self, index: u8) -> Field<'a, Offsets> {
            let (ty, elem_size, len) = define_vm_type_alias_region_helpers!(
                @array_info self.regions.pointer_type, $($fty)*
            );
            assert!(
                usize::from(index) < len,
                concat!(
                    "index out of bounds for `", stringify!($Name), "::",
                    stringify!($fname), "`"
                ),
            );
            let base = u32::from(
                self.regions.offsets.get_ptr_size().$snake().$fname()
            );
            let offset = base + u32::from(index) * elem_size;
            let flags = ir::MemFlagsData::trusted();
            $(
                let flags = define_vm_type_alias_region_helpers!(
                    @apply_attr flags, $fattr
                );
            )*
            Field::new(self.regions, VmType::$Name, offset, flags, ty)
        }
    };

    // Emit the accessor `struct` and per-field methods for one `VM*` type.
    //
    // Fields arrive pre-split into `{ $kind $fname [ $attrs... ] [ $fty... ] }`
    // groups (see `@munch` below); the terminal arm consumes those groups once
    // the struct body has been fully peeled apart.
    (@emit $Name:ident $snake:ident {
        $( { $fkind:ident $fname:ident [ $($fattr:tt)* ] [ $($fty:tt)* ] } )*
    }) => {
        #[doc = concat!(
            "An [`AliasRegions`] accessor for the fields of a `",
            stringify!($Name), "`."
        )]
        #[allow(
            dead_code,
            reason = "generated uniformly for all VM types; not all accessors are used yet"
        )]
        pub struct $Name<'a, Offsets> {
            regions: &'a mut AliasRegions<Offsets>,
        }

        #[allow(
            dead_code,
            reason = "generated uniformly for all VM types; not all accessors are used yet"
        )]
        impl<Offsets> AliasRegions<Offsets> {
            #[doc = concat!(
                "Get an accessor for the fields of a `", stringify!($Name), "`."
            )]
            pub fn $snake(&mut self) -> $Name<'_, Offsets> {
                $Name { regions: self }
            }
        }

        #[allow(
            dead_code,
            reason = "generated uniformly for all VM types; not all accessors are used yet"
        )]
        impl<'a, Offsets> $Name<'a, Offsets>
        where
            Offsets: GetPtrSize,
        {
            $(
                define_vm_type_alias_region_helpers!(
                    @method $Name $snake $fkind $fname [ $($fattr)* ] [ $($fty)* ]
                );
            )*
        }
    };

    // Peel fields off the raw struct body one at a time, accumulating completed
    // `{ $kind $fname [ $attrs... ] [ $fty... ] }` groups plus the pending
    // attributes for the field currently being parsed. `$kind` is `plain` or
    // `indexed`, and selects which `@method` arm emits the field's accessor.
    //
    // Splitting the body by hand (rather than matching `$fty:tt $(< $fgen:ty
    // >)?` within a repetition) is what lets each field's type reach the
    // `@field_ty` classifier as raw tokens, so it can require `Option`s to
    // specifically be `Option<VmPtr<_>>`.
    (@munch $Name:ident $snake:ident { $($groups:tt)* }) => {
        define_vm_type_alias_region_helpers!(@emit $Name $snake { $($groups)* });
    };
    // A field marked `#[aggregate]` is a composite (a nested struct or array)
    // so there is no single Cranelift type for such fields, so no accessors are
    // generated for them; interior accesses are computed from the field's
    // offset instead, which the generated `offsets::*` methods still provide.
    (@munch $Name:ident $snake:ident { $($groups:tt)* }
        $(#[doc = $fdoc:literal])* #[aggregate] $fvis:vis $fname:ident : $fty:ty , $($rest:tt)*
    ) => {
        define_vm_type_alias_region_helpers!(@munch $Name $snake { $($groups)* } $($rest)*);
    };
    // A field marked `#[indexed]` is a fixed-size array of a scalar type whose
    // elements are accessed individually by a compile-time constant index. It
    // has no single Cranelift type either, but its elements do, so it gets an
    // index-taking accessor.
    (@munch $Name:ident $snake:ident { $($groups:tt)* }
        $(#[doc = $fdoc:literal])* #[indexed] $fvis:vis $fname:ident : $($rest:tt)*
    ) => {
        define_vm_type_alias_region_helpers!(@munch_ty $Name $snake { $($groups)* } indexed $fname [] [] $($rest)*);
    };
    // Consume one field's attributes, visibility, and name, then collect its
    // type tokens.
    (@munch $Name:ident $snake:ident { $($groups:tt)* }
        $(# $fattr:tt)* $fvis:vis $fname:ident : $($rest:tt)*
    ) => {
        define_vm_type_alias_region_helpers!(@munch_ty $Name $snake { $($groups)* } plain $fname [ $($fattr)* ] [] $($rest)*);
    };
    // Accumulate one field's type tokens up to its terminating comma, then
    // append the completed group.
    (@munch_ty $Name:ident $snake:ident { $($groups:tt)* } $fkind:ident $fname:ident [ $($fattr:tt)* ] [ $($fty:tt)* ] , $($rest:tt)*) => {
        define_vm_type_alias_region_helpers!(@munch $Name $snake { $($groups)* { $fkind $fname [ $($fattr)* ] [ $($fty)* ] } } $($rest)*);
    };
    (@munch_ty $Name:ident $snake:ident { $($groups:tt)* } $fkind:ident $fname:ident [ $($fattr:tt)* ] [ $($fty:tt)* ] $tok:tt $($rest:tt)*) => {
        define_vm_type_alias_region_helpers!(@munch_ty $Name $snake { $($groups)* } $fkind $fname [ $($fattr)* ] [ $($fty)* $tok ] $($rest)*);
    };

    // Top-level entry: the list of `VM*` type definitions.
    ( $(
        $(#[doc = $sdoc:literal])*
        $(#[cfg($($scfg:tt)*)])?
        $(#[derive($($d:ident),*)])?
        #[repr($($repr:tt)*)]
        #[snake_name = $snake:ident]
        $svis:vis struct $Name:ident {
            $($body:tt)*
        }
    )* ) => {
        $(
            define_vm_type_alias_region_helpers!(@munch $Name $snake {} $($body)*);
        )*
    };
}
wasmtime_environ::for_each_vm_type!(define_vm_type_alias_region_helpers);

/// Define, for each of Wasmtime's vmctx types, an [`AliasRegions`] accessor
/// that returns a wrapper exposing a [`Field`]-returning method per field of
/// that vmctx.
///
/// For example, `alias_regions.vmctx().epoch_ptr()` is the `VMContext::epoch_ptr`
/// field, and `alias_regions.vmcomponent().callbacks(i)` is the `i`th element of
/// the `VMComponentContext`'s runtime-callbacks array.
///
/// A vmctx's `static` fields sit at offsets that depend only on the target
/// pointer size, so their accessors are available for any `Offsets: GetPtrSize`.
/// Its `dynamic` fields sit at offsets that additionally depend on the module or
/// component being compiled, so their accessors are only available when the
/// `AliasRegions` carries that vmctx's own fully-computed offsets. The exception
/// is a `dynamic` field marked `#[ptr_size_offset]`, whose offsets are derived
/// only from the pointer size; these live in the `Offsets: GetPtrSize` block as
/// well.
///
/// A field marked `#[aggregate]` gets no accessor, for the same reason it gets
/// none in [`define_vm_type_alias_region_helpers!`]: it has no single Cranelift
/// type. Instead, its interior is reached by rebasing a [`Field`] of the nested
/// `VM*` type onto the aggregate's own offset within the vmctx (see
/// [`Field::relative_to`]), which produces exactly the same alias region as
/// accessing that `VM*` type through a pointer would. For the few aggregates that
/// have no `VM*` type of their own, and hence no alias region of their own, there
/// are hand-written helpers below.
///
/// A field marked `#[pointee(..)]` additionally gets an accessor for what it
/// points *at*, which lives outside the vmctx. For example,
/// `alias_regions.vmctx().builtin_functions_array(i)` is the `i`th builtin in
/// the array that the `VMContext::builtin_functions` field points at.
#[allow(
    unused_macro_rules,
    reason = "entry shapes and marker attributes are handled uniformly for both \
              sections of both vmctx types, but not every combination occurs"
)]
macro_rules! define_vmctx_alias_region_helpers {
    // Classify a field type to its Cranelift `ir::Type`, given `$pt` (the target
    // pointer type as an `ir::Type`).
    (@field_ty $pt:expr, u32) => { ir::types::I32 };
    (@field_ty $pt:expr, VmPtr < $g:ident >) => { $pt };
    (@field_ty $pt:expr, VMSharedTypeIndex) => { ir::types::I32 };
    (@field_ty $pt:expr, AtomicU64) => { ir::types::I64 };
    (@field_ty $pt:expr, unsafe extern "C" fn) => { $pt };

    // Determine the Cranelift type a field is *accessed* as: the classification
    // of its `#[access_as = T]` type if it has one, and of its declared type
    // otherwise.
    (@access_ty $pt:expr, [ $($fty:tt)* ] []) => {
        define_vmctx_alias_region_helpers!(@field_ty $pt, $($fty)*)
    };
    (@access_ty $pt:expr, $fty:tt [ #[access_as = $($t:tt)*] $($rest:tt)* ]) => {
        define_vmctx_alias_region_helpers!(@field_ty $pt, $($t)*)
    };
    (@access_ty $pt:expr, $fty:tt [ # $skip:tt $($rest:tt)* ]) => {
        define_vmctx_alias_region_helpers!(@access_ty $pt, $fty [ $($rest)* ])
    };

    // Apply a field attribute to its access flags.
    (@apply_attr $flags:expr, [readonly]) => { $flags.with_readonly() };
    (@apply_attr $flags:expr, [can_move]) => { $flags.with_can_move() };
    (@apply_attr $flags:expr, [access_as = $($t:tt)*]) => { $flags };
    (@apply_attr $flags:expr, [ptr_size_offset]) => { $flags };
    (@apply_attr $flags:expr, [pointee( $($t:tt)* )]) => { $flags };

    // Compute a field's access flags and Cranelift type from its declared type
    // and marker attributes, and build the `Field` for it at `$offset`.
    (@field $Name:ident ($self:expr, $offset:expr) [ $($fty:tt)* ] [ $(# $fattr:tt)* ]) => {{
        let this = $self;
        let flags = ir::MemFlagsData::trusted();
        $( let flags = define_vmctx_alias_region_helpers!(@apply_attr flags, $fattr); )*
        let ty = define_vmctx_alias_region_helpers!(
            @access_ty this.regions.pointer_type, [ $($fty)* ] [ $(# $fattr)* ]
        );
        let offset = $offset;
        Field::new(this.regions, VmType::$Name, offset, flags, ty)
    }};

    // Scan a field's attributes for a `#[pointee(..)]` and emit the accessor it
    // describes, if any.
    (@pointee_methods $Offsets:tt [ ]) => {};
    (@pointee_methods $Offsets:tt [ #[pointee( $($p:tt)* )] $($rest:tt)* ]) => {
        define_vmctx_alias_region_helpers!(@pointee $Offsets $($p)*);
    };
    (@pointee_methods $Offsets:tt [ # $skip:tt $($rest:tt)* ]) => {
        define_vmctx_alias_region_helpers!(@pointee_methods $Offsets [ $($rest)* ]);
    };

    // An array of pointees.
    (@pointee [ $($Offsets:tt)* ]
        $(# $pattr:tt)* $Region:ident as $pname:ident [ $Index:ident ] : $($pty:tt)*
    ) => {
        #[doc = concat!(
            "Get the [`Field`] for the `index`th element of the out-of-line `",
            stringify!($Region), "` array.\n\nThe returned [`Field`] is relative \
             to the array's base pointer, not to the vmctx."
        )]
        pub fn $pname(self, index: $Index) -> Field<'a, $($Offsets)*> {
            let ty = define_vmctx_alias_region_helpers!(
                @field_ty self.regions.pointer_type, $($pty)*
            );
            let offset = index.vmctx_array_index().checked_mul(ty.bytes()).unwrap();
            let flags = ir::MemFlagsData::trusted();
            $( let flags = define_vmctx_alias_region_helpers!(@apply_attr flags, $pattr); )*
            Field::new(self.regions, VmType::$Region, offset, flags, ty)
        }
    };

    // A single pointee.
    (@pointee [ $($Offsets:tt)* ]
        $(# $pattr:tt)* $Region:ident as $pname:ident : $($pty:tt)*
    ) => {
        #[doc = concat!(
            "Get the [`Field`] for the out-of-line `", stringify!($Region),
            "`.\n\nThe returned [`Field`] is relative to the pointer to it, not \
             to the vmctx."
        )]
        pub fn $pname(self) -> Field<'a, $($Offsets)*> {
            let ty = define_vmctx_alias_region_helpers!(
                @field_ty self.regions.pointer_type, $($pty)*
            );
            let flags = ir::MemFlagsData::trusted();
            $( let flags = define_vmctx_alias_region_helpers!(@apply_attr flags, $pattr); )*
            Field::new(self.regions, VmType::$Region, 0, flags, ty)
        }
    };

    // ### `static` Section Entries

    (@static_entry $Name:ident $snake:ident align { $al:tt }) => {};

    // Aggregates get no accessor.
    (@static_entry $Name:ident $snake:ident $kind:ident { #[aggregate] $($rest:tt)* }) => {};

    (@static_entry $Name:ident $snake:ident field {
        $(# $fattr:tt)* $fname:ident : $($fty:tt)*
    }) => {
        #[doc = concat!(
            "Get the [`Field`] for the `", stringify!($fname), "` field of `",
            stringify!($Name), "`."
        )]
        pub fn $fname(self) -> Field<'a, Offsets> {
            let offset = u32::from(self.regions.offsets.get_ptr_size().$snake().$fname());
            define_vmctx_alias_region_helpers!(
                @field $Name (self, offset) [ $($fty)* ] [ $(# $fattr)* ]
            )
        }

        define_vmctx_alias_region_helpers!(@pointee_methods [ Offsets ] [ $(# $fattr)* ]);
    };

    // ### `dynamic` Section Entries Marked `#[ptr_size_offset]`
    //
    // These get their offsets from the pointer-size-only `offsets::VMFoo<P>`
    // wrapper, and don't require a full `VMOffsets` parameterization.

    (@ptr_size_entry $Name:ident $snake:ident field {
        #[ptr_size_offset] $(# $fattr:tt)* $fname:ident : $($fty:tt)*
    }) => {
        #[doc = concat!(
            "Get the [`Field`] for the `", stringify!($fname), "` field of `",
            stringify!($Name), "`."
        )]
        pub fn $fname(self) -> Field<'a, Offsets> {
            let offset = self.regions.offsets.get_ptr_size().$snake().$fname();
            define_vmctx_alias_region_helpers!(
                @field $Name (self, offset) [ $($fty)* ] [ $(# $fattr)* ]
            )
        }
    };

    (@ptr_size_entry $Name:ident $snake:ident array {
        #[ptr_size_offset] $(# $fattr:tt)* $fname:ident [ $count:ident ; $Index:ident ] : $($fty:tt)*
    }) => {
        #[doc = concat!(
            "Get the [`Field`] for the `index`th element of `", stringify!($Name),
            "`'s `", stringify!($fname), "` array.\n\nThis is not bounds checked: \
             the array's length depends on the module or component being compiled, \
             which is precisely what this accessor does not require knowing."
        )]
        pub fn $fname(self, index: $Index) -> Field<'a, Offsets> {
            let offset = self.regions.offsets.get_ptr_size().$snake().$fname(index);
            define_vmctx_alias_region_helpers!(
                @field $Name (self, offset) [ $($fty)* ] [ $(# $fattr)* ]
            )
        }
    };

    (@ptr_size_entry $Name:ident $snake:ident $kind:ident $entry:tt) => {};

    // ### `dynamic` Section Entries

    // Entries marked `#[ptr_size_offset]` were already handled above; emitting
    // them here too would be a duplicate definition.
    (@dynamic_entry $Name:ident $Offsets:tt $kind:ident {
        #[ptr_size_offset] $($rest:tt)*
    }) => {};

    (@dynamic_entry $Name:ident $Offsets:tt align { $al:tt }) => {};

    // Aggregates get no accessor.
    (@dynamic_entry $Name:ident $Offsets:tt $kind:ident { #[aggregate] $($rest:tt)* }) => {};

    (@dynamic_entry $Name:ident [ $($Offsets:tt)* ] array {
        $(# $fattr:tt)* $fname:ident [ $count:ident ; $Index:ident ] : $($fty:tt)*
    }) => {
        #[doc = concat!(
            "Get the [`Field`] for the `index`th element of `", stringify!($Name),
            "`'s `", stringify!($fname), "` array.\n\nPanics if `index` is out of \
             bounds for this vmctx."
        )]
        pub fn $fname(self, index: $Index) -> Field<'a, $($Offsets)*> {
            let offset = self.regions.offsets.$fname().at(index);
            define_vmctx_alias_region_helpers!(
                @field $Name (self, offset) [ $($fty)* ] [ $(# $fattr)* ]
            )
        }

        define_vmctx_alias_region_helpers!(
            @pointee_methods [ $($Offsets)* ] [ $(# $fattr)* ]
        );
    };

    // A single field, whether unconditionally present (`field`) or only
    // conditionally (`optional`).
    (@dynamic_entry $Name:ident [ $($Offsets:tt)* ] $kind:ident {
        $(# $fattr:tt)* $fname:ident $([ if $flag:ident ])? : $($fty:tt)*
    }) => {
        #[doc = concat!(
            "Get the [`Field`] for the `", stringify!($fname), "` field of `",
            stringify!($Name), "`."
            $(, "\n\nPanics if `", stringify!($flag), "` is false, in which case \
                 this field is not present at all.")?
        )]
        pub fn $fname(self) -> Field<'a, $($Offsets)*> {
            let offset = self.regions.offsets.$fname();
            define_vmctx_alias_region_helpers!(
                @field $Name (self, offset) [ $($fty)* ] [ $(# $fattr)* ]
            )
        }

        define_vmctx_alias_region_helpers!(
            @pointee_methods [ $($Offsets)* ] [ $(# $fattr)* ]
        );
    };

    // Emit the `impl` block holding the accessors for the dynamically-positioned
    // fields.
    (@dynamic_impl $Name:ident [ $($Offsets:tt)* ] $OffsetsTt:tt {
        $($kind:ident $entry:tt)*
    }) => {
        #[allow(
            dead_code,
            reason = "generated uniformly for every field; not all fields are \
                      accessed by compiled code"
        )]
        impl<'a> $Name<'a, $($Offsets)*> {
            $(
                define_vmctx_alias_region_helpers!(
                    @dynamic_entry $Name $OffsetsTt $kind $entry
                );
            )*
        }
    };

    // Emit the accessor `struct` and both `impl` blocks for one vmctx type.
    (@emit $Name:ident $snake:ident $Offsets:tt
        static { $($skind:ident $sentry:tt)* }
        dynamic { $($dkind:ident $dentry:tt)* }
    ) => {
        #[doc = concat!(
            "An [`AliasRegions`] accessor for the fields of a `", stringify!($Name),
            "`."
        )]
        pub struct $Name<'a, Offsets> {
            regions: &'a mut AliasRegions<Offsets>,
        }

        impl<Offsets> AliasRegions<Offsets> {
            #[doc = concat!(
                "Get an accessor for the fields of a `", stringify!($Name), "`."
            )]
            pub fn $snake(&mut self) -> $Name<'_, Offsets> {
                $Name { regions: self }
            }
        }

        // A statically-positioned field's offset depends only on the target
        // pointer size, so these accessors work with any `Offsets`.
        #[allow(
            dead_code,
            reason = "generated uniformly for every field; not all fields are \
                      accessed by compiled code"
        )]
        impl<'a, Offsets> $Name<'a, Offsets>
        where
            Offsets: GetPtrSize,
        {
            $( define_vmctx_alias_region_helpers!(@static_entry $Name $snake $skind $sentry); )*
            $( define_vmctx_alias_region_helpers!(@ptr_size_entry $Name $snake $dkind $dentry); )*
        }

        // A dynamically-positioned field's offset depends on the module or
        // component being compiled, so these accessors require this vmctx's own
        // fully-computed offsets.
        define_vmctx_alias_region_helpers!(
            @dynamic_impl $Name $Offsets $Offsets { $($dkind $dentry)* }
        );
    };

    // Map each vmctx type to the offsets type that computes its
    // dynamically-positioned fields' offsets.
    (@one VMContext $snake:ident static { $($stat:tt)* } dynamic { $($dyn:tt)* }) => {
        define_vmctx_alias_region_helpers!(@emit VMContext $snake [ VMOffsets<u8> ]
            static { $($stat)* } dynamic { $($dyn)* });
    };
    (@one VMComponentContext $snake:ident static { $($stat:tt)* } dynamic { $($dyn:tt)* }) => {
        define_vmctx_alias_region_helpers!(
            @emit VMComponentContext $snake [ VMComponentOffsets<u8> ]
            static { $($stat)* } dynamic { $($dyn)* }
        );
    };

    // Top-level entry.
    ( $(
        {
            $Name:ident $snake:ident
            static { $($stat:tt)* }
            dynamic { $($dyn:tt)* }
        }
    )* ) => {
        $(
            define_vmctx_alias_region_helpers!(@one $Name $snake
                static { $($stat)* } dynamic { $($dyn)* });
        )*
    };
}
wasmtime_environ::for_each_vmctx_type!(define_vmctx_alias_region_helpers);

impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Create a new `AliasRegions`.
    pub fn new(offsets: Offsets) -> Self {
        Self {
            pointer_type: ir::Type::int_with_byte_size(offsets.get_ptr_size().size().into())
                .unwrap(),
            offsets,
            cache: std::collections::HashMap::default(),
        }
    }

    /// The Cranelift type of a pointer on the target being compiled for.
    pub fn pointer_type(&self) -> ir::Type {
        self.pointer_type
    }

    /// The pointer size of the target being compiled for, used to compute the
    /// layout of Wasmtime's vmctx types.
    pub fn ptr_size(&self) -> &Offsets::Ptr {
        self.offsets.get_ptr_size()
    }

    /// Get the alias region for accesses into the GC heap.
    pub fn gc_heap_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::GcHeap)
    }

    /// Get the alias region shared by all memories that cross a module boundary
    /// and whose definition we do not statically know.
    pub fn public_memory_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::PublicMemory)
    }

    /// Get the alias region for accessing a particular defined memory.
    pub fn defined_memory_region(
        &mut self,
        func: &mut ir::Function,
        module: StaticModuleIndex,
        index: DefinedMemoryIndex,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DefinedMemory { module, index })
    }

    /// Get the alias region shared by all tables that cross a module boundary
    /// and whose definition we do not statically know.
    pub fn public_table_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::PublicTable)
    }

    /// Get the alias region for accessing a particular defined table.
    pub fn defined_table_region(
        &mut self,
        func: &mut ir::Function,
        module: StaticModuleIndex,
        index: DefinedTableIndex,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DefinedTable { module, index })
    }

    /// Get the alias region shared by all globals that cross a module boundary
    /// and whose definition we do not statically know.
    pub fn public_global_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::PublicGlobal)
    }

    /// Get the alias region for accessing a particular defined global.
    pub fn defined_global_region(
        &mut self,
        func: &mut ir::Function,
        module: StaticModuleIndex,
        index: DefinedGlobalIndex,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DefinedGlobal { module, index })
    }
}

/// `VMStoreContext`-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    fn vmstore_context_region(&mut self, func: &mut ir::Function, offset: u32) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMStoreContext,
                offset,
            },
        )
    }

    /// Get a `Load` of the GC heap base pointer (`VMStoreContext::gc_heap.base`).
    ///
    /// The caller supplies the base flags because whether the base pointer is
    /// `readonly`/`can_move` depends on the GC heap's tunables.
    pub fn vmstore_context_gc_heap_base_load(
        &mut self,
        func: &mut ir::Function,
        base_flags: ir::MemFlagsData,
    ) -> Load {
        let offset = self
            .offsets
            .get_ptr_size()
            .vm_store_context()
            .gc_heap_base()
            .into();
        let region = self.vmstore_context_region(func, offset);
        Load {
            offset,
            flags: base_flags.with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the GC heap base pointer (`VMStoreContext::gc_heap.base`).
    ///
    /// The caller supplies the base flags because whether the base pointer is
    /// `readonly`/`can_move` depends on the GC heap's tunables.
    pub fn vmstore_context_gc_heap_base(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_gc_heap_base_load(cursor.func, base_flags)
            .emit(cursor, vmstore_ctx)
    }

    /// Get a `Load` of the GC heap bound (`VMStoreContext::gc_heap.current_length`).
    pub fn vmstore_context_gc_heap_current_length_load(&mut self, func: &mut ir::Function) -> Load {
        let offset = self
            .offsets
            .get_ptr_size()
            .vm_store_context()
            .gc_heap_current_length()
            .into();
        let region = self.vmstore_context_region(func, offset);
        Load {
            offset,
            flags: ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the GC heap bound (`VMStoreContext::gc_heap.current_length`).
    pub fn vmstore_context_gc_heap_current_length(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_gc_heap_current_length_load(cursor.func)
            .emit(cursor, vmstore_ctx)
    }

    /// Get the alias region for the `VMStoreContext::stack_chain` field.
    ///
    /// The `VMStackChain` is two pointers wide and is emitted by the stack
    /// switching `VMStackChain` load/store helpers, which take a region
    /// argument; this provides that region.
    pub fn vmstore_context_stack_chain_region(
        &mut self,
        func: &mut ir::Function,
    ) -> ir::AliasRegion {
        let offset = self.offsets.get_ptr_size().vm_store_context().stack_chain();
        self.vmstore_context_region(func, offset.into())
    }
}

/// Component-specific methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Get the alias region for the `ValRaw` array used to marshal arguments
    /// and results across the array calling convention used by various
    /// trampolines.
    pub fn host_val_raw_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::HostValRaw,
                offset: 0,
            },
        )
    }
}

/// Passive data and element segment-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Get the alias region for the runtime `ValRaw` storage of passive element
    /// segments (written by passive-element initialization and read by
    /// `table.init` and `array.{new,init}_elem`).
    pub fn element_segment_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::ElementSegment)
    }

    /// Get the alias region for the bytes of data segments (read by
    /// `memory.init` and `array.{new,init}_data`).
    pub fn data_segment_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DataSegment)
    }
}

/// Unsafe intrinsic-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Perform an unsafe intrinsic's load.
    pub fn unsafe_intrinsic_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        addr: ir::Value,
    ) -> ir::Value {
        let region = self.region(cursor.func, AliasRegionKey::UnsafeIntrinsicMemory);
        cursor
            .ins()
            .load(ty, base_flags.with_alias_region(Some(region)), addr, 0)
    }

    /// Perform an unsafe intrinsic's store.
    pub fn unsafe_intrinsic_store(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        val: ir::Value,
        addr: ir::Value,
    ) {
        let region = self.region(cursor.func, AliasRegionKey::UnsafeIntrinsicMemory);
        cursor
            .ins()
            .store(base_flags.with_alias_region(Some(region)), val, addr, 0);
    }
}

/// `VMComponentContext` fields that are not simply one of the layout's own
/// fields, and so are not generated by [`define_vmctx_alias_region_helpers!`].
impl AliasRegions<VMComponentOffsets<u8>> {
    /// Get the [`Field`] for the `callee` of the `index`th lowering in the
    /// `VMComponentContext`'s `lowerings` array.
    pub fn vmcomponent_lowering_callee(
        &mut self,
        index: LoweredIndex,
    ) -> Field<'_, VMComponentOffsets<u8>> {
        let offset = self.offsets.lowering_callee(index);
        self.vmlowering_field(offset)
    }

    /// Get the [`Field`] for the host data of the `index`th lowering in the
    /// `VMComponentContext`'s `lowerings` array.
    pub fn vmcomponent_lowering_data(
        &mut self,
        index: LoweredIndex,
    ) -> Field<'_, VMComponentOffsets<u8>> {
        let offset = self.offsets.lowering_data(index);
        self.vmlowering_field(offset)
    }

    /// Get the [`Field`] at `offset` within a `VMLowering` inlined into the
    /// `VMComponentContext`.
    ///
    /// Unlike the other aggregates inlined into a vmctx, `VMLowering` is not one
    /// of the types defined by `for_each_vm_type!`, and so has no alias region of
    /// its own to rebase onto the aggregate's offset. Its fields are therefore
    /// part of the containing `VMComponentContext`'s region, keyed by their
    /// offset within it.
    fn vmlowering_field(&mut self, offset: u32) -> Field<'_, VMComponentOffsets<u8>> {
        let ty = self.pointer_type;
        Field::new(
            self,
            VmType::VMComponentContext,
            offset,
            ir::MemFlagsData::trusted(),
            ty,
        )
    }
}

/// Stack-switching and continuation-object methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Load `VMStackLimits::stack_limit`.
    pub fn stack_limit(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        stack_limits: ir::Value,
    ) -> ir::Value {
        let offset = self.offsets.get_ptr_size().vmstack_limits_stack_limit();
        let region = self.vmcontref_region(cursor.func);
        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            stack_limits,
            i32::from(offset),
        )
    }

    /// Load `VMStackLimits::last_wasm_entry_fp`.
    pub fn last_wasm_entry_fp(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        stack_limits: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmstack_limits_last_wasm_entry_fp();
        let region = self.vmcontref_region(cursor.func);
        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            stack_limits,
            i32::from(offset),
        )
    }

    /// Load `VMStackLimits::last_wasm_entry_sp`.
    pub fn last_wasm_entry_sp(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        stack_limits: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmstack_limits_last_wasm_entry_sp();
        let region = self.vmcontref_region(cursor.func);
        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            stack_limits,
            i32::from(offset),
        )
    }

    /// Load `VMStackLimits::last_wasm_entry_trap_handler`.
    pub fn last_wasm_entry_trap_handler(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        stack_limits: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmstack_limits_last_wasm_entry_trap_handler();
        let region = self.vmcontref_region(cursor.func);
        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            stack_limits,
            i32::from(offset),
        )
    }

    /// Region for a continuation-reference object and its inline
    /// sub-structures.
    ///
    /// A `VMContRef` (and its inline `VMCommonStackInformation` /
    /// `VMStackLimits` / `VMHostArray` headers) is reached through a `*mut
    /// VMContRef`.
    ///
    /// A single region covers the whole object: this is coarse but sound, and
    /// keeps every field of the object disjoint from linear memory, the vmctx,
    /// the store context, etc...
    pub fn vmcontref_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMContRef,
                offset: 0,
            },
        )
    }

    /// Region for a continuation's stack memory: its payload/handler data
    /// buffers and the control-context records stored on the continuation
    /// stack.
    ///
    /// These are distinct from the `VMContRef` object itself (which only holds
    /// pointers to them).
    pub fn continuation_stack_memory_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::ContinuationStackMemory,
                offset: 0,
            },
        )
    }
}

/// Debug-assert that every CLIF memory instruction in `func` carries an alias
/// region.
///
/// The Wasm-to-CLIF translation in `crates/cranelift/*` tags 100% of the memory
/// instructions it emits (loads, stores, and atomics) with an alias region.
/// This helper upholds that invariant; it is called at each
/// `FunctionBuilder::finalize` site, after finalization so that the stack-map
/// spills inserted by the safepoint pass (tagged via the
/// `FunctionBuilder::make_stack_map_alias_region` hook) are checked too.
pub(crate) fn debug_assert_all_mem_insts_have_alias_regions(func: &ir::Function) {
    if cfg!(debug_assertions) {
        for block in func.layout.blocks() {
            for inst in func.layout.block_insts(block) {
                // Only loads, stores, and atomics access memory. Some non-memory
                // instructions (e.g. `bitcast`) also carry `MemFlags` purely for
                // lane/byte-order and legitimately have no alias region, so they
                // are excluded here.
                let opcode = func.dfg.insts[inst].opcode();
                if !opcode.can_load() && !opcode.can_store() {
                    continue;
                }
                if let Some(flags) = func.dfg.insts[inst].memflags() {
                    debug_assert!(
                        func.dfg.mem_flags[flags].alias_region().is_some(),
                        "CLIF memory instruction emitted without an alias region: {}",
                        func.dfg.display_inst(inst),
                    );
                }
            }
        }
    }
}
