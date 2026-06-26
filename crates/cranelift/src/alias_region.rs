use crate::translate::Load;
use core::fmt;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{self, InstBuilder as _},
};
use wasmtime_environ::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, GetPtrSize, GlobalIndex,
    MemoryIndex, OwnedMemoryIndex, PtrSize as _, RuntimeDataIndex, StaticModuleIndex, TableIndex,
    TagIndex, VMOffsets,
    component::{
        LoweredIndex, ResourceIndex, RuntimeCallbackIndex, RuntimeComponentInstanceIndex,
        RuntimeMemoryIndex, RuntimePostReturnIndex, VMComponentOffsets,
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum VmType {
    VMContext,
    VMStoreContext,
    VMMemoryDefinition,
    VMTableDefinition,
    VMComponentContext,
    VMDrcHeapData,
    VMCopyingHeapData,
    VMNullHeapData,
    VMDeferredThread,
}

/// A key that uniquely identifies an alias region across an entire compilation.
///
/// This is used to assign stable `user_id`s to `AliasRegionData` entries so
/// that alias regions can be deduplicated during inlining.
///
/// The key encodes into a single `u32` with the following layout:
/// `[ kind: 5 bits | data: 27 bits ]`
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
}

impl AliasRegionKey {
    const KIND_BITS: u32 = 4;
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

    const VM_CONTEXT_KIND: u32 = Self::new_kind(0b0000);
    const VM_STORE_CONTEXT_KIND: u32 = Self::new_kind(0b0001);
    const IMPORTED_MEMORY_KIND: u32 = Self::new_kind(0b0010);
    const DEFINED_MEMORY_KIND: u32 = Self::new_kind(0b0011);
    const IMPORTED_TABLE_KIND: u32 = Self::new_kind(0b0100);
    const DEFINED_TABLE_KIND: u32 = Self::new_kind(0b0101);
    const IMPORTED_GLOBAL_KIND: u32 = Self::new_kind(0b0110);
    const DEFINED_GLOBAL_KIND: u32 = Self::new_kind(0b0111);
    const GC_HEAP_KIND: u32 = Self::new_kind(0b1000);
    const VM_MEMORY_DEFINITION_KIND: u32 = Self::new_kind(0b1001);
    const VM_TABLE_DEFINITION_KIND: u32 = Self::new_kind(0b1010);
    const VM_COMPONENT_CONTEXT_KIND: u32 = Self::new_kind(0b1011);
    const VM_DRC_HEAP_DATA_KIND: u32 = Self::new_kind(0b1100);
    const VM_COPYING_HEAP_DATA_KIND: u32 = Self::new_kind(0b1101);
    const VM_NULL_HEAP_DATA_KIND: u32 = Self::new_kind(0b1110);
    const VM_DEFERRED_THREAD_KIND: u32 = Self::new_kind(0b1111);

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
                    VmType::VMComponentContext => Self::VM_COMPONENT_CONTEXT_KIND,
                    VmType::VMDrcHeapData => Self::VM_DRC_HEAP_DATA_KIND,
                    VmType::VMCopyingHeapData => Self::VM_COPYING_HEAP_DATA_KIND,
                    VmType::VMNullHeapData => Self::VM_NULL_HEAP_DATA_KIND,
                    VmType::VMDeferredThread => Self::VM_DEFERRED_THREAD_KIND,
                };
                kind | (offset & Self::OFFSET_MASK)
            }
            AliasRegionKey::PublicMemory => Self::IMPORTED_MEMORY_KIND,
            AliasRegionKey::DefinedMemory { module, index } => {
                debug_assert_eq!(
                    module.as_u32() & !Self::MODULE_MASK >> Self::MODULE_OFFSET,
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
                    module.as_u32() & !Self::MODULE_MASK >> Self::MODULE_OFFSET,
                    0
                );
                debug_assert_eq!(index.as_u32() & !Self::INDEX_MASK, 0);
                Self::DEFINED_TABLE_KIND | (module.as_u32() << Self::MODULE_OFFSET) | index.as_u32()
            }
            AliasRegionKey::PublicGlobal => Self::IMPORTED_GLOBAL_KIND,
            AliasRegionKey::DefinedGlobal { module, index } => {
                debug_assert_eq!(
                    module.as_u32() & !Self::MODULE_MASK >> Self::MODULE_OFFSET,
                    0
                );
                debug_assert_eq!(index.as_u32() & !Self::INDEX_MASK, 0);
                Self::DEFINED_GLOBAL_KIND
                    | (module.as_u32() << Self::MODULE_OFFSET)
                    | index.as_u32()
            }
            AliasRegionKey::GcHeap => Self::GC_HEAP_KIND,
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

    /// Get the alias region for the given key.
    fn region(&mut self, func: &mut ir::Function, key: AliasRegionKey) -> ir::AliasRegion {
        *self
            .cache
            .entry(key)
            .or_insert_with(|| func.dfg.alias_regions.insert(key.into()))
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
    /// Load the imported tag's `VMTagImport::vmctx` field from the `*mut
    /// VMContext`.
    pub fn vmctx_vmtag_import_vmctx(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        tag: TagIndex,
    ) -> ir::Value {
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmtag_import_vmctx(tag),
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
        self.vmctx_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmtag_import_index(tag),
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
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmtag_import_from(tag),
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
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmfunction_import_vmctx(func),
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
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            self.offsets.vmctx_vmfunction_import_wasm_call(func),
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
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            mem_vmctx_offset,
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
        self.vmctx_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            mem_index_offset,
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
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            table_vmctx_offset,
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
        self.vmctx_load(
            cursor,
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            table_index_offset,
        )
    }

    /// Get a `Load` for the imported table's `VMTableImport::from` field (a
    /// `*mut VMTableDefinition`) out of a `*mut VMContext`.
    pub fn vmctx_vmtable_from_load(&mut self, func: &mut ir::Function, table: TableIndex) -> Load {
        let offset = self.offsets.vmctx_vmtable_from(table);
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

    /// Load the imported global's address (`VMGlobalImport::from`) out of the
    /// `*mut VMContext`.
    pub fn vmctx_vmglobal_import_from(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmctx: ir::Value,
        global: GlobalIndex,
    ) -> ir::Value {
        let from_offset = self.offsets.vmctx_vmglobal_import_from(global);
        self.vmctx_load(
            cursor,
            self.pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            from_offset,
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

    /// Get a `Load` for an inlined-in-the-`vmctx` `VMMemoryDefinition`'s `base`
    /// field.
    pub fn vmctx_vmmemory_definition_base_load(
        &mut self,
        func: &mut ir::Function,
        memory: OwnedMemoryIndex,
        base_flags: ir::MemFlagsData,
    ) -> Load {
        let field = self.offsets.ptr.vmmemory_definition_base();
        let region = self.vmmemory_definition_region(func, field.into());
        Load {
            offset: self.offsets.vmctx_vmmemory_definition_base(memory),
            flags: base_flags.with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load an inlined-in-the-`vmctx` `VMMemoryDefinition`'s `base` field.
    pub fn vmctx_vmmemory_definition_base(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        memory: OwnedMemoryIndex,
        base_flags: ir::MemFlagsData,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmctx_vmmemory_definition_base_load(cursor.func, memory, base_flags)
            .emit(cursor, vmctx)
    }

    /// Get a `Load` for an inlined-in-the-`vmctx` `VMMemoryDefinition`'s `current_length`
    /// field.
    pub fn vmctx_vmmemory_definition_current_length_load(
        &mut self,
        func: &mut ir::Function,
        memory: OwnedMemoryIndex,
    ) -> Load {
        let field = self.offsets.ptr.vmmemory_definition_current_length();
        let region = self.vmmemory_definition_region(func, field.into());
        Load {
            offset: self
                .offsets
                .vmctx_vmmemory_definition_current_length(memory),
            flags: ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Load an inlined-in-the-`vmctx` `VMMemoryDefinition`'s `current_length` field.
    pub fn vmctx_vmmemory_definition_current_length(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        memory: OwnedMemoryIndex,
        vmctx: ir::Value,
    ) -> ir::Value {
        self.vmctx_vmmemory_definition_current_length_load(cursor.func, memory)
            .emit(cursor, vmctx)
    }

    fn vmtable_definition_region(
        &mut self,
        func: &mut ir::Function,
        offset: u32,
    ) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMTableDefinition,
                offset,
            },
        )
    }

    /// Get a `Load` for an inlined-in-the-`vmctx` `VMTableDefinition`'s `base`
    /// field.
    pub fn vmctx_vmtable_definition_base_load(
        &mut self,
        func: &mut ir::Function,
        table: DefinedTableIndex,
        base_flags: ir::MemFlagsData,
    ) -> Load {
        // NB: The region is keyed on the field's offset within the
        // `VMTableDefinition`, not the `vmctx`, so that defined
        // (`vmctx`-inlined) and imported (via-pointer) tables share one region
        // per field.
        let field = self.offsets.vmtable_definition_base();
        let region = self.vmtable_definition_region(func, field.into());

        Load {
            offset: self.offsets.vmctx_vmtable_definition_base(table),
            flags: base_flags.with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Get a `Load` for an inlined-in-the-`vmctx` `VMTableDefinition`'s
    /// `current_elements` field.
    ///
    /// The caller supplies `ty` because the field's width depends on the table
    /// elements' type.
    pub fn vmctx_vmtable_definition_current_elements_load(
        &mut self,
        func: &mut ir::Function,
        table: DefinedTableIndex,
        ty: ir::Type,
    ) -> Load {
        // See note in `vmctx_vmtable_definition_base_load`.
        let field = self.offsets.vmtable_definition_current_elements();
        let region = self.vmtable_definition_region(func, field.into());

        Load {
            offset: self
                .offsets
                .vmctx_vmtable_definition_current_elements(table),
            flags: ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ty,
        }
    }

    /// Get a `Load` for the `VMTableDefinition::base` field reached through a
    /// `*mut VMTableDefinition` (an imported table), for use in a
    /// `VmctxLoadChain`.
    pub fn vmtable_definition_base_load(
        &mut self,
        func: &mut ir::Function,
        base_flags: ir::MemFlagsData,
    ) -> Load {
        let offset = self.offsets.vmtable_definition_base().into();
        let region = self.vmtable_definition_region(func, offset);
        Load {
            offset,
            flags: base_flags.with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Get a `Load` for the `VMTableDefinition::current_elements` field reached
    /// through a `*mut VMTableDefinition` (an imported table).
    ///
    /// The caller supplies `ty` because the field's width depends on the table
    /// elements' type.
    pub fn vmtable_definition_current_elements_load(
        &mut self,
        func: &mut ir::Function,
        ty: ir::Type,
    ) -> Load {
        let offset = self.offsets.vmtable_definition_current_elements().into();
        let region = self.vmtable_definition_region(func, offset);
        Load {
            offset,
            flags: ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ty,
        }
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

/// `VMMemoryDefinition`-related methods.
///
/// The `base` and `current_length` fields are reached either directly through
/// the `vmctx` (for an owned, inline memory) or through a `*mut
/// VMMemoryDefinition` (for a shared/imported memory). Both cases must share
/// one region per field, so the region is keyed on the field's offset *within*
/// the `VMMemoryDefinition` regardless of how the field is addressed; this is
/// required for soundness under inlining (cf. `memory_alias_region`).
impl<Offsets> AliasRegions<Offsets>
where
    Offsets: GetPtrSize,
{
    fn vmmemory_definition_region(
        &mut self,
        func: &mut ir::Function,
        offset: u32,
    ) -> ir::AliasRegion {
        self.region(
            func,
            AliasRegionKey::Vm {
                ty: VmType::VMMemoryDefinition,
                offset,
            },
        )
    }

    /// Create a `Load` for the `VMMemoryDefinition::base` field, for use in a
    /// `VmctxLoadChain`.
    pub fn vmmemory_definition_base_load(
        &mut self,
        func: &mut ir::Function,
        base_flags: ir::MemFlagsData,
    ) -> Load {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmmemory_definition_base()
            .into();
        let region = self.vmmemory_definition_region(func, offset);
        Load {
            offset,
            flags: base_flags.with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Create a `Load` for the `VMMemoryDefinition::current_length` field, for
    /// use in a `VmctxLoadChain`.
    pub fn vmmemory_definition_current_length_load(&mut self, func: &mut ir::Function) -> Load {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmmemory_definition_current_length()
            .into();
        let region = self.vmmemory_definition_region(func, offset);
        Load {
            offset,
            flags: ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ty: self.pointer_type,
        }
    }

    /// Emit a load of the `VMMemoryDefinition::base` field from the given `vmmemory_definition`.
    pub fn vmmemory_definition_base(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmmemory_definition: ir::Value,
    ) -> ir::Value {
        self.vmmemory_definition_base_load(cursor.func, ir::MemFlagsData::trusted())
            .emit(cursor, vmmemory_definition)
    }

    /// Emit a (non-atomic) load of the `VMMemoryDefinition::current_length`
    /// field from the given `vmmemory_definition`.
    pub fn vmmemory_definition_current_length(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmmemory_definition: ir::Value,
    ) -> ir::Value {
        self.vmmemory_definition_current_length_load(cursor.func)
            .emit(cursor, vmmemory_definition)
    }

    /// Emit an atomic load of the `VMMemoryDefinition::current_length` field out
    /// of a `*mut VMMemoryDefinition`.
    pub fn vmmemory_definition_current_length_atomic(
        &mut self,
        cursor: &mut FuncCursor<'_>,
        vmmemory_definition: ir::Value,
    ) -> ir::Value {
        let offset = self
            .offsets
            .get_ptr_size()
            .vmmemory_definition_current_length();
        let region = self.vmmemory_definition_region(cursor.func, offset.into());
        let offset = cursor.ins().iconst(self.pointer_type, i64::from(offset));
        let ptr = cursor.ins().iadd(vmmemory_definition, offset);
        cursor.ins().atomic_load(
            self.pointer_type,
            ir::MemFlagsData::trusted().with_alias_region(Some(region)),
            ptr,
        )
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
