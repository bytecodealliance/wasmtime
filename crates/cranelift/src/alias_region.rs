use crate::translate::Load;
use core::fmt;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{self, InstBuilder as _},
};
use wasmtime_environ::{
    BuiltinFunctionIndex, DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex,
    GetPtrSize, GlobalIndex, MemoryIndex, ModuleInternedTypeIndex, PtrSize as _, RuntimeDataIndex,
    StaticModuleIndex, TableIndex, TagIndex, VMOffsets,
    component::{
        ComponentBuiltinFunctionIndex, LoweredIndex, ResourceIndex, RuntimeCallbackIndex,
        RuntimeComponentInstanceIndex, RuntimeMemoryIndex, RuntimePostReturnIndex,
        VMComponentOffsets,
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

    /// An imported or exported memory access (shared across all
    /// imported/exported memories).
    PublicMemory,

    /// A defined memory access.
    DefinedMemory {
        /// The static module index.
        module: StaticModuleIndex,
        /// The defined memory index within the module.
        index: DefinedMemoryIndex,
    },

    /// An imported or exported table access (shared across all
    /// imported/exported tables).
    PublicTable,

    /// A defined table access.
    DefinedTable {
        /// The static module index.
        module: StaticModuleIndex,
        /// The defined table index within the module.
        index: DefinedTableIndex,
    },

    /// An imported or exported global access (shared across all
    /// imported/exported globals).
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
    /// independent of [`Field::offset`]: adjusting the load/store offset (e.g.
    /// to make the access relative to the `vmctx` rather than to a pointer to
    /// the containing structure) must not change which alias region the access
    /// belongs to.
    key: AliasRegionKey,
    /// The offset added to the base value when emitting a load or store of this
    /// field.
    ///
    /// Initially the field's offset *within* its containing `VM*` type; callers
    /// may adjust it via [`Field::offset`].
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
    pub fn readonly(&mut self) -> &mut Self {
        self.flags.set_readonly();
        self
    }

    /// Mark accesses of this field as `can_move`.
    ///
    /// See the note on [`Field::readonly`].
    pub fn can_move(&mut self) -> &mut Self {
        self.flags = self.flags.with_can_move();
        self
    }

    /// Cast this field to the given type.
    ///
    /// This can be used, for example, to cast a `Field` that points to a
    /// `VMGlobalDefinition`'s storage (a `[u8; 16]` represented as
    /// `ir::types::I8X16`) to the global's actual Wasm type's representation
    /// (`ir::types::I32` for a Wasm `i32`).
    #[allow(
        dead_code,
        reason = "part of the general `Field` API; not all fields are cast yet"
    )]
    pub fn cast(&mut self, ty: ir::Type) -> &mut Self {
        self.ty = ty;
        self
    }

    /// Get a mutable reference to the offset at which this field's load or store
    /// will be emitted, relative to its base value.
    ///
    /// The offset starts out as the field's offset *within* its containing
    /// `VM*` type, so accesses are relative to a pointer to that structure. To
    /// make an access relative to something else, adjust the offset: for
    /// example, add the structure's own offset within the `vmctx` when the
    /// structure is inlined into the `vmctx` (as for an owned memory's
    /// `VMMemoryDefinition`) so that the access is relative to the `vmctx`
    /// directly.
    ///
    /// Adjusting the offset does not change the field's alias region.
    pub fn offset(&mut self) -> &mut u32 {
        &mut self.offset
    }

    /// Get-or-create this field's alias region and mix it into this field's
    /// base flags.
    fn flags_with_region(&mut self, func: &mut ir::Function) -> ir::MemFlagsData {
        let region = self.regions.region(func, self.key);
        self.flags.with_alias_region(Some(region))
    }

    /// Get a deferred [`Load`] descriptor for this field, for use in a
    /// `VmctxLoadChain`.
    ///
    /// The load is emitted at [`Field::offset`] relative to its base value. By
    /// default that is the field's offset within its containing `VM*`
    /// structure, so the load is relative to a pointer to that structure; to
    /// make it relative to something else (e.g. the `vmctx`, when the structure
    /// is inlined into the `vmctx`), adjust [`Field::offset`] first.
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
    #[allow(
        dead_code,
        reason = "part of the general `Field` API; not all fields are stored to yet"
    )]
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
#[allow(
    unused_macro_rules,
    reason = "the `#[readonly]`/`#[can_move]` marker arms are generated \
              uniformly but not exercised until a VM type uses those markers"
)]
macro_rules! define_vm_type_alias_region_helpers {
    // Classify a field type to its Cranelift `ir::Type`, given `$pt` (the
    // target pointer type as an `ir::Type`).
    (@field_ty $pt:expr, VmPtr < u8 >) => { $pt };
    (@field_ty $pt:expr, AtomicUsize) => { $pt };
    (@field_ty $pt:expr, usize) => { $pt };
    (@field_ty $pt:expr, [u8; 16]) => { ir::types::I8X16 };
    (@field_ty $pt:expr, VMSharedTypeIndex) => { ir::types::I32 };

    // Apply a field attribute to its access flags. The `#[readonly]` and
    // `#[can_move]` markers map to the corresponding `MemFlagsData` builder
    // methods; doc comments have no effect on flags; any other attribute is a
    // compile error.
    (@apply_attr $flags:expr, [readonly]) => { $flags.with_readonly() };
    (@apply_attr $flags:expr, [can_move]) => { $flags.with_can_move() };
    (@apply_attr $flags:expr, [doc = $d:literal]) => { $flags };

    // Top-level entry: the list of `VM*` type definitions.
    ( $(
        $(#[doc = $sdoc:literal])*
        $(#[derive($($d:ident),*)])?
        #[repr($($repr:tt)*)]
        #[snake_name = $snake:ident]
        $svis:vis struct $Name:ident {
            $(
                $( # $fattr:tt )*
                $fvis:vis $fname:ident : $fty:tt $(< $fgen:tt >)? ,
            )*
        }
    )* ) => {
        $(
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
                            @field_ty self.regions.pointer_type, $fty $(< $fgen >)?
                        );
                        Field::new(self.regions, VmType::$Name, offset, flags, ty)
                    }
                )*
            }
        )*
    };
}
wasmtime_environ::for_each_vm_type!(define_vm_type_alias_region_helpers);

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

    /// Get the alias region for accesses into the GC heap.
    pub fn gc_heap_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::GcHeap)
    }

    /// Get the alias region for an imported or exported memory access (shared
    /// across all imported/exported memories).
    pub fn public_memory_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::PublicMemory)
    }

    /// Get the alias region for accessing a defined memory that is not
    /// exported.
    pub fn defined_memory_region(
        &mut self,
        func: &mut ir::Function,
        module: StaticModuleIndex,
        index: DefinedMemoryIndex,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DefinedMemory { module, index })
    }

    /// Get the alias region for an imported or exported table access (shared
    /// across all imported/exported memories).
    pub fn public_table_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::PublicTable)
    }

    /// Get the alias region for accessing a defined table that is not
    /// exported.
    pub fn defined_table_region(
        &mut self,
        func: &mut ir::Function,
        module: StaticModuleIndex,
        index: DefinedTableIndex,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DefinedTable { module, index })
    }

    /// Get the alias region for an imported or exported global access (shared
    /// across all imported/exported memories).
    pub fn public_global_region(&mut self, func: &mut ir::Function) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::PublicGlobal)
    }

    /// Get the alias region for accessing a defined global that is not
    /// exported.
    pub fn defined_global_region(
        &mut self,
        func: &mut ir::Function,
        module: StaticModuleIndex,
        index: DefinedGlobalIndex,
    ) -> ir::AliasRegion {
        self.region(func, AliasRegionKey::DefinedGlobal { module, index })
    }
}

/// `VMContext`-related methods that are valid for any `VMContext`, regardless
/// of its particular `VMOffsets`.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Get the alias region for the given offset into the `VMContext`.
    fn vmctx_region(&mut self, func: &mut ir::Function, offset: u32) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMContext,
                offset,
            },
        )
    }

    fn vmctx_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
        offset: u32,
    ) -> ir::Value {
        let region = self.vmctx_region(cursor.func, offset);
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            vmctx,
            i32::try_from(offset).unwrap(),
        )
    }

    fn vmctx_store(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
        offset: u32,
        val: ir::Value,
    ) {
        let region = self.vmctx_region(cursor.func, offset);
        cursor.ins().store(
            base_flags.with_alias_region(Some(region)),
            val,
            vmctx,
            i32::try_from(offset).unwrap(),
        );
    }

    /// Load the `VMContext::magic` field.
    pub fn vmctx_magic(&mut self, cursor: &mut FuncCursor<'_>, vmctx: ir::Value) -> ir::Value {
        self.vmctx_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.get_ptr_size().vmctx_magic().into(),
        )
    }

    /// Load the `*mut VMStoreContext` value out of the given `*mut VMContext`.
    pub fn vmctx_store_context(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmctx_store_context_load(cursor.func)
            .emit(cursor, vmctx)
    }

    /// Get a `Load` for the `*mut VMStoreContext` value out of a `*mut VMContext`.
    pub fn vmctx_store_context_load(&mut self, func: &mut ir::Function) -> Load {
        let offset = u32::from(self.offsets.get_ptr_size().vmctx_store_context());
        let region = self.vmctx_region(func, offset);
        Load {
            offset,
            flags: ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the `*mut i64` epoch pointer out of the given `*mut VMContext`.
    pub fn vmctx_epoch_ptr(&mut self, cursor: &mut FuncCursor<'_>, vmctx: ir::Value) -> ir::Value {
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.get_ptr_size().vmctx_epoch_ptr().into(),
        )
    }

    /// Load the base pointer of the `[VMSharedTypeIndex]` array out of the
    /// given `*mut VMContext`.
    pub fn vmctx_shared_type_ids_array(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.get_ptr_size().vmctx_type_ids_array().into(),
        )
    }

    /// Load the collector's heap data pointer out of the `*mut VMContext`.
    pub fn vmctx_gc_heap_data(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.get_ptr_size().vmctx_gc_heap_data().into(),
        )
    }

    /// Load the base pointer to the builtin-functions array from a `*mut
    /// VMContext`.
    pub fn vmctx_builtin_functions(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets
                .get_ptr_size()
                .vmcontext_builtin_functions()
                .into(),
        )
    }
}

/// `VMContext`-related methods that are specific to a particular Wasm module's
/// `VMOffsets`.
impl AliasRegions<VMOffsets<u8>> {
    /// Like `vmctx_load`, but tags the load with a per-import alias region
    /// rather than the coarse `VMContext` region. Used for fields of the
    /// `VM*Import` structs inlined into the `VMContext`.
    fn vmimport_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
        vmctx_offset: u32,
        field_offset: u32,
        vm_type: VmType,
    ) -> ir::Value {
        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: vm_type,
                offset: field_offset,
            },
        );
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            vmctx,
            i32::try_from(vmctx_offset).unwrap(),
        )
    }

    /// Load the imported tag's `VMTagImport::vmctx` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmtag_import_vmctx(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        tag: TagIndex,
    ) -> ir::Value {
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmtag_import_vmctx(tag),
            self.offsets.vmtag_import_vmctx().into(),
            VmType::VMTagImport,
        )
    }

    /// Load the imported tag's `VMTagImport::index` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmtag_import_index(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        tag: TagIndex,
    ) -> ir::Value {
        self.vmimport_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmtag_import_index(tag),
            self.offsets.vmtag_import_index().into(),
            VmType::VMTagImport,
        )
    }

    /// Load the imported tag's `VMTagImport::from` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmtag_import_from(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        tag: TagIndex,
    ) -> ir::Value {
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmtag_import_from(tag),
            self.offsets.vmtag_import_from().into(),
            VmType::VMTagImport,
        )
    }

    /// Load the import function's `VMFunctionImport::vmctx` field from the
    /// `*mut VMContext`.
    pub fn vmctx_vmfunction_import_vmctx(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        func: FuncIndex,
    ) -> ir::Value {
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmfunction_import_vmctx(func),
            self.offsets.vmfunction_import_vmctx().into(),
            VmType::VMFunctionImport,
        )
    }

    /// Load the import function's `VMFunctionImport::wasm_call` field from the
    /// `*mut VMContext`.
    pub fn vmctx_vmfunction_import_wasm_call(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        func: FuncIndex,
    ) -> ir::Value {
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmfunction_import_wasm_call(func),
            self.offsets.vmfunction_import_wasm_call().into(),
            VmType::VMFunctionImport,
        )
    }

    /// Load the imported memory's `VMMemoryImport::vmctx` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmmemory_import_vmctx(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        memory: MemoryIndex,
    ) -> ir::Value {
        let mem_offset = self.offsets.vmctx_vmmemory_import(memory);
        let mem_vmctx_offset = mem_offset + u32::from(self.offsets.vmmemory_import_vmctx());
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            mem_vmctx_offset,
            self.offsets.vmmemory_import_vmctx().into(),
            VmType::VMMemoryImport,
        )
    }

    /// Load the imported memory's `VMMemoryImport::index` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmmemory_import_index(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        memory: MemoryIndex,
    ) -> ir::Value {
        let mem_offset = self.offsets.vmctx_vmmemory_import(memory);
        let mem_index_offset = mem_offset + u32::from(self.offsets.vmmemory_import_index());
        self.vmimport_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            mem_index_offset,
            self.offsets.vmmemory_import_index().into(),
            VmType::VMMemoryImport,
        )
    }

    /// Load the imported memory's `VMMemoryImport::from` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmmemory_import_from(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        memory: MemoryIndex,
    ) -> ir::Value {
        self.vmctx_vmmemory_import_from_load(cursor.func, memory)
            .emit(cursor, vmctx)
    }

    /// Get a `Load` for the imported memory's `VMMemoryImport::from` field from
    /// a `*mut VMContext`.
    pub fn vmctx_vmmemory_import_from_load(
        &mut self,
        func: &mut ir::Function,
        memory: MemoryIndex,
    ) -> Load {
        let mem_offset = self.offsets.vmctx_vmmemory_import(memory);
        let offset = mem_offset + u32::from(self.offsets.vmmemory_import_from());
        let region = self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMMemoryImport,
                offset: self.offsets.vmmemory_import_from().into(),
            },
        );
        Load {
            offset,
            flags: ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the imported table's `VMTableImport::vmctx` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmtable_import_vmctx(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        table: TableIndex,
    ) -> ir::Value {
        let table_offset = self.offsets.vmctx_vmtable_import(table);
        let table_vmctx_offset = table_offset + u32::from(self.offsets.vmtable_import_vmctx());
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            table_vmctx_offset,
            self.offsets.vmtable_import_vmctx().into(),
            VmType::VMTableImport,
        )
    }

    /// Load the imported table's `VMTableImport::index` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmtable_import_index(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        table: TableIndex,
    ) -> ir::Value {
        let table_offset = self.offsets.vmctx_vmtable_import(table);
        let table_index_offset = table_offset + u32::from(self.offsets.vmtable_import_index());
        self.vmimport_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            table_index_offset,
            self.offsets.vmtable_import_index().into(),
            VmType::VMTableImport,
        )
    }

    /// Get a `Load` for the imported table's `VMTableImport::from` field (a
    /// `*mut VMTableDefinition`) out of a `*mut VMContext`.
    pub fn vmctx_vmtable_from_load(&mut self, func: &mut ir::Function, table: TableIndex) -> Load {
        let offset = self.offsets.vmctx_vmtable_from(table);
        let region = self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMTableImport,
                offset: self.offsets.vmtable_import_from().into(),
            },
        );
        Load {
            offset,
            flags: ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the imported global's address (`VMGlobalImport::from`) out of the
    /// `*mut VMContext`.
    pub fn vmctx_vmglobal_import_from(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        global: GlobalIndex,
    ) -> ir::Value {
        let from_offset = self.offsets.vmctx_vmglobal_import_from(global);
        self.vmimport_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            from_offset,
            self.offsets.vmglobal_import_from().into(),
            VmType::VMGlobalImport,
        )
    }

    /// Load the defined memory's `*mut VMMemoryDefinition` out of the `*mut
    /// VMContext`.
    pub fn vmctx_vmmemory_pointer(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        memory: DefinedMemoryIndex,
    ) -> ir::Value {
        self.vmctx_vmmemory_pointer_load(cursor.func, memory)
            .emit(cursor, vmctx)
    }

    /// Get a `Load` for the defined memory's `*mut VMMemoryDefinition` out of a
    /// `*mut VMContext`.
    pub fn vmctx_vmmemory_pointer_load(
        &mut self,
        func: &mut ir::Function,
        memory: DefinedMemoryIndex,
    ) -> Load {
        let offset = self.offsets.vmctx_vmmemory_pointer(memory);
        let region = self.vmctx_region(func, offset);
        Load {
            offset,
            flags: ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the base of the given runtime data out of the `*mut VMContext`.
    pub fn vmctx_runtime_data_base(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        runtime_data: RuntimeDataIndex,
    ) -> ir::Value {
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.vmctx_runtime_data_base(runtime_data),
        )
    }

    /// Load the length of the given runtime data out of the `*mut VMContext`.
    pub fn vmctx_runtime_data_length(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        runtime_data: RuntimeDataIndex,
    ) -> ir::Value {
        self.vmctx_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.vmctx_runtime_data_length(runtime_data),
        )
    }

    /// Load the length of the given runtime data out of the `*mut VMContext`.
    pub fn store_vmctx_runtime_data_length(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        runtime_data: RuntimeDataIndex,
        new_length: ir::Value,
    ) {
        self.vmctx_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.vmctx_runtime_data_length(runtime_data),
            new_length,
        )
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

    fn vmstore_context_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        vmstore_ctx: ir::Value,
        offset: u32,
    ) -> ir::Value {
        let region = self.vmstore_context_region(cursor.func, offset);
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            vmstore_ctx,
            i32::try_from(offset).unwrap(),
        )
    }

    fn vmstore_context_store(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        vmstore_ctx: ir::Value,
        offset: u32,
        val: ir::Value,
    ) {
        let region = self.vmstore_context_region(cursor.func, offset);
        cursor.ins().store(
            base_flags.with_alias_region(Some(region)),
            val,
            vmstore_ctx,
            i32::try_from(offset).unwrap(),
        );
    }

    /// Load a pointer to the `*mut T` store data from a `*mut VMStoreContext`.
    pub fn vmstore_context_store_data(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_store_data()
                .into(),
        )
    }

    /// Load the `VMStoreContext::execution_version` field.
    pub fn vmstore_context_execution_version(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            ir::types::I64,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_execution_version()
                .into(),
        )
    }

    /// Store the `VMStoreContext::execution_version` field.
    pub fn store_vmstore_context_execution_version(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        new_version: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_execution_version()
                .into(),
            new_version,
        )
    }

    /// Load the `VMStoreContext::fuel_consumed` field.
    pub fn vmstore_context_fuel_consumed(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            ir::types::I64,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_fuel_consumed()
                .into(),
        )
    }

    /// Store the `VMStoreContext::fuel_consumed` field.
    pub fn store_vmstore_context_fuel_consumed(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        fuel_consumed: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_fuel_consumed()
                .into(),
            fuel_consumed,
        )
    }

    /// Load the `VMStoreContext::epoch_deadline` field.
    pub fn vmstore_context_epoch_deadline(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            ir::types::I64,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_epoch_deadline()
                .into(),
        )
    }

    /// Get a `Load` for the `VmStoreContext::stack_limits` field.
    pub fn vmstore_context_stack_limit_load(&mut self, func: &mut ir::Function) -> Load {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmstore_context_stack_limit()
            .into();
        let region = self.vmstore_context_region(func, offset);
        Load {
            offset,
            flags: ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load the `VMStoreContext::stack_limit` field.
    pub fn vmstore_context_stack_limit(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_stack_limit_load(cursor.func)
            .emit(cursor, vmstore_ctx)
    }

    /// Store the `VMStoreContext::stack_limit` field.
    pub fn store_vmstore_context_stack_limit(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        stack_limit: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_stack_limit()
                .into(),
            stack_limit,
        )
    }

    /// Load the `VMStoreContext::current_thread` field (the JIT-visible
    /// deferred-thread pointer; see `VMLazyThread`).
    pub fn vmstore_context_current_thread(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_current_thread()
                .into(),
        )
    }

    /// Store the `VMStoreContext::current_thread` field.
    pub fn store_vmstore_context_current_thread(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        new_thread: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_current_thread()
                .into(),
            new_thread,
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
            .vmstore_context_gc_heap_base()
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
            .vmstore_context_gc_heap_current_length()
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

    /// Load the `VMStoreContext::last_wasm_entry_fp` field.
    pub fn vmstore_context_last_wasm_entry_fp(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_last_wasm_entry_fp()
                .into(),
        )
    }

    /// Store the `VMStoreContext::last_wasm_entry_fp` field.
    pub fn store_vmstore_context_last_wasm_entry_fp(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        fp: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_last_wasm_entry_fp()
                .into(),
            fp,
        )
    }

    /// Store the `VMStoreContext::last_wasm_entry_sp` field.
    pub fn store_vmstore_context_last_wasm_entry_sp(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        sp: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_last_wasm_entry_sp()
                .into(),
            sp,
        )
    }

    /// Store the `VMStoreContext::last_wasm_entry_trap_handler` field.
    pub fn store_vmstore_context_last_wasm_entry_trap_handler(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        trap_handler: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_last_wasm_entry_trap_handler()
                .into(),
            trap_handler,
        )
    }

    /// Store the `VMStoreContext::last_wasm_exit_trampoline_fp` field.
    pub fn store_vmstore_context_last_wasm_exit_trampoline_fp(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        fp: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_last_wasm_exit_trampoline_fp()
                .into(),
            fp,
        )
    }

    /// Store the `VMStoreContext::last_wasm_exit_pc` field.
    pub fn store_vmstore_context_last_wasm_exit_pc(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        pc: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_last_wasm_exit_pc()
                .into(),
            pc,
        )
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
        let offset = self.offsets.get_ptr_size().vmstore_context_stack_chain();
        self.vmstore_context_region(func, offset.into())
    }

    /// Load a `VMStoreContext` component-context slot.
    ///
    /// The slot is indexed by a compile-time constant, so the alias region is
    /// keyed on the precise per-slot offset.
    pub fn vmstore_context_component_context_slot(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        vmstore_ctx: ir::Value,
        slot: u8,
    ) -> ir::Value {
        self.vmstore_context_load(
            cursor,
            ty,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_component_context_slot(slot)
                .into(),
        )
    }

    /// Store a `VMStoreContext` component-context slot.
    ///
    /// The slot is indexed by a compile-time constant, so the alias region is
    /// keyed on the precise per-slot offset.
    pub fn store_vmstore_context_component_context_slot(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmstore_ctx: ir::Value,
        slot: u8,
        val: ir::Value,
    ) {
        self.vmstore_context_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            self.offsets
                .get_ptr_size()
                .vmstore_context_component_context_slot(slot)
                .into(),
            val,
        )
    }
}

/// `VMDeferredThread`-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    fn vmdeferred_thread_region(
        &mut self,
        func: &mut ir::Function,
        offset: u32,
    ) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMDeferredThread,
                offset,
            },
        )
    }

    fn vmdeferred_thread_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        vmdeferred_thread_ptr: ir::Value,
        offset: u32,
    ) -> ir::Value {
        let region = self.vmdeferred_thread_region(cursor.func, offset);
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            vmdeferred_thread_ptr,
            i32::try_from(offset).unwrap(),
        )
    }

    fn vmdeferred_thread_store(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        vmdeferred_thread_ptr: ir::Value,
        offset: u32,
        val: ir::Value,
    ) {
        let region = self.vmdeferred_thread_region(cursor.func, offset);
        cursor.ins().store(
            base_flags.with_alias_region(Some(region)),
            val,
            vmdeferred_thread_ptr,
            i32::try_from(offset).unwrap(),
        );
    }

    /// Load `VMDeferredThread::parent` (the current thread this frame replaced).
    pub fn vmdeferred_thread_parent(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
    ) -> ir::Value {
        self.vmdeferred_thread_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_parent()
                .into(),
        )
    }

    /// Store `VMDeferredThread::parent`.
    pub fn store_vmdeferred_thread_parent(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
        parent: ir::Value,
    ) {
        self.vmdeferred_thread_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_parent()
                .into(),
            parent,
        )
    }

    /// Store `VMDeferredThread::caller_instance`.
    pub fn store_vmdeferred_thread_caller_instance(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
        caller_instance: ir::Value,
    ) {
        self.vmdeferred_thread_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_caller_instance()
                .into(),
            caller_instance,
        )
    }

    /// Store `VMDeferredThread::callee_async`.
    pub fn store_vmdeferred_thread_callee_async(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
        callee_async: ir::Value,
    ) {
        self.vmdeferred_thread_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_callee_async()
                .into(),
            callee_async,
        )
    }

    /// Store `VMDeferredThread::callee_instance`.
    pub fn store_vmdeferred_thread_callee_instance(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
        callee_instance: ir::Value,
    ) {
        self.vmdeferred_thread_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_callee_instance()
                .into(),
            callee_instance,
        )
    }

    /// Load `VMDeferredThread::saved_context[i]` (a saved `context.{get,set}`
    /// slot).
    pub fn vmdeferred_thread_saved_context(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
        i: u8,
    ) -> ir::Value {
        self.vmdeferred_thread_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_saved_context(i)
                .into(),
        )
    }

    /// Store `VMDeferredThread::saved_context[i]`.
    pub fn store_vmdeferred_thread_saved_context(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmdeferred_thread_ptr: ir::Value,
        i: u8,
        val: ir::Value,
    ) {
        self.vmdeferred_thread_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmdeferred_thread_ptr,
            self.offsets
                .get_ptr_size()
                .vmdeferred_thread_saved_context(i)
                .into(),
            val,
        )
    }
}

/// `VMFuncRef`-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    fn vmfuncref_region(&mut self, func: &mut ir::Function, offset: u32) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMFuncRef,
                offset,
            },
        )
    }

    /// Load the `VMFuncRef::type_index` field out of a `*const VMFuncRef`.
    pub fn vmfuncref_type_index(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        funcref: ir::Value,
    ) -> ir::Value {
        let offset = self.offsets.get_ptr_size().vm_func_ref_type_index();
        let region = self.vmfuncref_region(cursor.func, offset.into());
        let ty = ir::Type::int_with_byte_size(
            self.offsets
                .get_ptr_size()
                .size_of_vmshared_type_index()
                .into(),
        )
        .unwrap();
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            funcref,
            i32::from(offset),
        )
    }

    /// Load the `VMFuncRef::wasm_call` field out of a `*const VMFuncRef`.
    ///
    /// The caller supplies the base flags because this load may carry an
    /// optional trap code for the null-funcref case.
    pub fn vmfuncref_wasm_call(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        funcref: ir::Value,
    ) -> ir::Value {
        let offset = self.offsets.get_ptr_size().vm_func_ref_wasm_call();
        let region = self.vmfuncref_region(cursor.func, offset.into());
        cursor.ins().load(
            self.pointer_type,
            base_flags.with_alias_region(Some(region)),
            funcref,
            i32::from(offset),
        )
    }

    /// Load the `VMFuncRef::vmctx` field out of a `*const VMFuncRef`.
    pub fn vmfuncref_vmctx(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        funcref: ir::Value,
    ) -> ir::Value {
        let offset = self.offsets.get_ptr_size().vm_func_ref_vmctx();
        let region = self.vmfuncref_region(cursor.func, offset.into());
        cursor.ins().load(
            self.pointer_type,
            base_flags.with_alias_region(Some(region)),
            funcref,
            i32::from(offset),
        )
    }

    /// Load the `array_call` field of the `VMFuncRef` inlined in a
    /// `VMArrayCallHostFuncContext`.
    pub fn vmarray_call_host_func_context_array_call(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        host_func_ctx: ir::Value,
    ) -> ir::Value {
        let func_ref = self
            .offsets
            .get_ptr_size()
            .vmarray_call_host_func_context_func_ref();
        let field = self.offsets.get_ptr_size().vm_func_ref_array_call();
        let region = self.vmfuncref_region(cursor.func, field.into());
        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            host_func_ctx,
            i32::from(func_ref) + i32::from(field),
        )
    }
}

/// `[VMSharedTypeIndex]`-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Load a `VMSharedTypeIndex` element out of the `[VMSharedTypeIndex]`
    /// array pointed at by the `VMContext::type_ids_array` field.
    pub fn type_ids_array_element(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        array: ir::Value,
        ty: ModuleInternedTypeIndex,
    ) -> ir::Value {
        let load_ty = ir::Type::int_with_byte_size(
            self.offsets
                .get_ptr_size()
                .size_of_vmshared_type_index()
                .into(),
        )
        .unwrap();

        let offset = ty.as_u32().checked_mul(load_ty.bytes()).unwrap();
        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::TypeIdsArray,
                offset,
            },
        );

        cursor.ins().load(
            load_ty,
            ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            array,
            i32::try_from(offset).unwrap(),
        )
    }
}

/// Epoch counter-related methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Dereference the epoch pointer (an `*const AtomicU64` previously loaded
    /// out of the vmctx's `epoch_ptr` field) to read the current epoch counter.
    pub fn epoch_counter(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        epoch_ptr: ir::Value,
    ) -> ir::Value {
        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::EpochCounter,
                offset: 0,
            },
        );
        cursor.ins().load(
            ir::types::I64,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            epoch_ptr,
            0,
        )
    }
}

/// Builtin-functions array methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Load a host function pointer element out of the builtin-functions array
    /// (`VMContext::builtin_functions`)
    pub fn builtin_functions_array_element(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        array: ir::Value,
        builtin: BuiltinFunctionIndex,
    ) -> ir::Value {
        let offset = builtin
            .index()
            .checked_mul(self.pointer_type.bytes())
            .unwrap();

        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::BuiltinFunctionsArray,
                offset,
            },
        );

        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            array,
            i32::try_from(offset).unwrap(),
        )
    }
}

/// Component builtin-functions array methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Load a host function pointer element out of the component
    /// builtin-functions array (`VMComponentContext::builtins`).
    pub fn component_builtin_functions_array_element(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        array: ir::Value,
        builtin: ComponentBuiltinFunctionIndex,
    ) -> ir::Value {
        let offset = builtin
            .index()
            .checked_mul(self.pointer_type.bytes())
            .unwrap();

        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::ComponentBuiltinFunctionsArray,
                offset,
            },
        );

        cursor.ins().load(
            self.pointer_type,
            ir::MemFlagsData::trusted()
                .with_readonly()
                .with_can_move()
                .with_alias_region(Some(region)),
            array,
            i32::try_from(offset).unwrap(),
        )
    }

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

/// `VMComponentContext`-related methods, used when compiling component
/// trampolines.
impl AliasRegions<VMComponentOffsets<u8>> {
    fn vmcomponent_region(&mut self, func: &mut ir::Function, offset: u32) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMComponentContext,
                offset,
            },
        )
    }

    fn vmcomponent_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
        offset: u32,
    ) -> ir::Value {
        let region = self.vmcomponent_region(cursor.func, offset);
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            vmctx,
            i32::try_from(offset).unwrap(),
        )
    }

    fn vmcomponent_store(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
        offset: u32,
        val: ir::Value,
    ) {
        let region = self.vmcomponent_region(cursor.func, offset);
        cursor.ins().store(
            base_flags.with_alias_region(Some(region)),
            val,
            vmctx,
            i32::try_from(offset).unwrap(),
        );
    }

    /// Load a lowering's host-data pointer from the `VMComponentContext`.
    pub fn vmcomponent_lowering_data(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        index: LoweredIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.lowering_data(index),
        )
    }

    /// Load a lowering's host callee pointer from the `VMComponentContext`.
    pub fn vmcomponent_lowering_callee(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        index: LoweredIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.lowering_callee(index),
        )
    }

    /// Load the current task's `may_block` flag from the `VMComponentContext`.
    pub fn vmcomponent_task_may_block(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly(),
            vmctx,
            self.offsets.task_may_block(),
        )
    }

    /// Store the current task's `may_block` flag into the `VMComponentContext`.
    pub fn store_vmcomponent_task_may_block(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        val: ir::Value,
    ) {
        self.vmcomponent_store(
            cursor,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.task_may_block(),
            val,
        )
    }

    /// Load a resource's destructor function pointer from the
    /// `VMComponentContext`.
    pub fn vmcomponent_resource_destructor(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        index: ResourceIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly(),
            vmctx,
            self.offsets.resource_destructor(index),
        )
    }

    /// Load a runtime memory's `*mut VMMemoryDefinition` from the
    /// `VMComponentContext`.
    pub fn vmcomponent_runtime_memory(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        index: RuntimeMemoryIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.runtime_memory(index),
        )
    }

    /// Load a runtime callback function pointer from the `VMComponentContext`.
    pub fn vmcomponent_runtime_callback(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        index: RuntimeCallbackIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.runtime_callback(index),
        )
    }

    /// Load a runtime post-return function pointer from the
    /// `VMComponentContext`.
    pub fn vmcomponent_runtime_post_return(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        index: RuntimePostReturnIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.runtime_post_return(index),
        )
    }

    /// Load the base pointer of the component builtins array from the
    /// `VMComponentContext`.
    pub fn vmcomponent_builtins(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly(),
            vmctx,
            self.offsets.builtins(),
        )
    }

    /// Load a component instance's `may_leave` flag from the `VMComponentContext`.
    pub fn vmcomponent_instance_may_leave(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        instance: RuntimeComponentInstanceIndex,
    ) -> ir::Value {
        self.vmcomponent_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted(),
            vmctx,
            self.offsets.may_leave(instance),
        )
    }
}

/// `VMComponentContext`-related methods that need to be generic over `Offsets`
/// due to the call context.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    /// Load a field at `offset` within a `VMComponentContext`.
    pub fn vmcomponent_context_generic_load(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        ty: ir::Type,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
        offset: u32,
    ) -> ir::Value {
        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::VMComponentContext,
                offset,
            },
        );
        cursor.ins().load(
            ty,
            base_flags.with_alias_region(Some(region)),
            vmctx,
            i32::try_from(offset).unwrap(),
        )
    }
}

/// Methods for the collectors' private heap-data structs.
///
/// Each struct is a separate allocation reached through a `*mut _` stored in the
/// `VMContext`. Their fields are *not* GC heap locations, so they are tagged with
/// the owning struct's own region (keyed on the field offset within the struct)
/// rather than the `GcHeap` region. These helpers emit the field load/store.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    fn vmdrc_heap_data_region(&mut self, func: &mut ir::Function, offset: u32) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMDrcHeapData,
                offset,
            },
        )
    }

    /// Emit a load of the DRC over-approximated-stack-roots list head, given the
    /// a `*mut VMDrcHeapData`.
    pub fn vmdrc_heap_data_over_approximated_stack_roots(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        drc_heap_data: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmdrc_heap_data_over_approximated_stack_roots();
        let region = self.vmdrc_heap_data_region(cursor.func, offset.into());
        cursor.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            drc_heap_data,
            i32::from(offset),
        )
    }

    /// Emit a store to the DRC over-approximated-stack-roots list head.
    pub fn store_vmdrc_heap_data_over_approximated_stack_roots(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        drc_heap_data: ir::Value,
        val: ir::Value,
    ) {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmdrc_heap_data_over_approximated_stack_roots();
        let region = self.vmdrc_heap_data_region(cursor.func, offset.into());
        cursor.ins().store(
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            val,
            drc_heap_data,
            i32::from(offset),
        );
    }

    /// Emit a load of the current over-approximated-stack-roots list length.
    pub fn vmdrc_heap_data_current_over_approximated_stack_roots_len(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        drc_heap_data: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmdrc_heap_data_current_over_approximated_stack_roots_len();
        let region = self.vmdrc_heap_data_region(cursor.func, offset.into());
        cursor.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            drc_heap_data,
            i32::from(offset),
        )
    }

    /// Emit a store to the current over-approximated-stack-roots list length.
    pub fn store_vmdrc_heap_data_current_over_approximated_stack_roots_len(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        drc_heap_data: ir::Value,
        len: ir::Value,
    ) {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmdrc_heap_data_current_over_approximated_stack_roots_len();
        let region = self.vmdrc_heap_data_region(cursor.func, offset.into());
        cursor.ins().store(
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            len,
            drc_heap_data,
            i32::from(offset),
        );
    }

    /// Emit a load of the over-approximated-stack-roots list length after the
    /// last GC.
    pub fn vmdrc_heap_data_over_approximated_stack_roots_len_after_last_gc(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        drc_heap_data: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmdrc_heap_data_over_approximated_stack_roots_len_after_last_gc();
        let region = self.vmdrc_heap_data_region(cursor.func, offset.into());
        cursor.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            drc_heap_data,
            i32::from(offset),
        )
    }

    fn vmcopying_heap_data_region(
        &mut self,
        func: &mut ir::Function,
        offset: u32,
    ) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMCopyingHeapData,
                offset,
            },
        )
    }

    /// Emit a load of the copying collector's bump pointer.
    pub fn vmcopying_heap_data_bump_ptr(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        copying_heap_data: ir::Value,
    ) -> ir::Value {
        let offset = self.offsets.get_ptr_size().vmcopying_heap_data_bump_ptr();
        let region = self.vmcopying_heap_data_region(cursor.func, offset.into());
        cursor.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            copying_heap_data,
            i32::from(offset),
        )
    }

    /// Emit a store to the copying collector's bump pointer.
    pub fn store_vmcopying_heap_data_bump_ptr(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        copying_heap_data: ir::Value,
        val: ir::Value,
    ) {
        let offset = self.offsets.get_ptr_size().vmcopying_heap_data_bump_ptr();
        let region = self.vmcopying_heap_data_region(cursor.func, offset.into());
        cursor.ins().store(
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            val,
            copying_heap_data,
            i32::from(offset),
        );
    }

    /// Emit a load of the copying collector's active-space-end pointer.
    pub fn vmcopying_heap_data_active_space_end(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        copying_heap_data: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmcopying_heap_data_active_space_end();
        let region = self.vmcopying_heap_data_region(cursor.func, offset.into());
        cursor.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            copying_heap_data,
            i32::from(offset),
        )
    }

    /// Emit a load of the null collector's bump finger (the first and only field
    /// of its heap data, at offset 0).
    pub fn vmnull_heap_data_bump_finger(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        null_collector_heap_data: ir::Value,
    ) -> ir::Value {
        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::VMNullHeapData,
                offset: 0,
            },
        );
        cursor.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            null_collector_heap_data,
            0,
        )
    }

    /// Emit a store to the null collector's bump finger.
    pub fn store_vmnull_heap_data_bump_finger(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        null_collector_heap_data: ir::Value,
        val: ir::Value,
    ) {
        let region = self.region(
            cursor.func,
            AliasRegionKey::Vm {
                ty: VmType::VMNullHeapData,
                offset: 0,
            },
        );
        cursor.ins().store(
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            val,
            null_collector_heap_data,
            0,
        );
    }
}

/// Stack-switching and continuation-object methods.
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
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
