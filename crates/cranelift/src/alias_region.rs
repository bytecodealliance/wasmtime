use crate::translate::Load;
use core::fmt;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{self, InstBuilder as _},
};
use wasmtime_environ::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, GetPtrSize, GlobalIndex,
    MemoryIndex, PtrSize as _, RuntimeDataIndex, StaticModuleIndex, TableIndex, TagIndex,
    VMOffsets,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum VmType {
    VMContext,
    VMStoreContext,

    #[allow(
        dead_code,
        reason = "used when tagging `VMMemoryDefinition` fields in upcoming commits"
    )]
    VMMemoryDefinition,
}

/// A key that uniquely identifies an alias region across an entire compilation.
///
/// This is used to assign stable `user_id`s to `AliasRegionData` entries so
/// that alias regions can be deduplicated during inlining.
///
/// The key encodes into a single `u32` with the following layout:
/// `[ kind: 4 bits | data: 28 bits ]`
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

    /// Get the region for loading from a `*mut VMContext` at the given offset.
    ///
    /// XXX: This is ONLY for use with `ir::GlobalValue`s, all other uses should
    /// instead use a helper method that actually emits the full load
    /// instruction instead (e.g. `AliasRegions::vmctx_store_context`).
    pub fn vmctx_region_for_use_in_ir_global(
        &mut self,
        func: &mut ir::Function,
        offset: u32,
    ) -> ir::AliasRegion {
        self.vmctx_region(func, offset)
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
}
