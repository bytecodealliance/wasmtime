use core::fmt;
use cranelift_codegen::ir;
use wasmtime_environ::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, StaticModuleIndex,
};

/// A key that uniquely identifies an alias region across an entire compilation.
///
/// This is used to assign stable `user_id`s to `AliasRegionData` entries so
/// that alias regions can be deduplicated during inlining.
///
/// The key encodes into a single `u32` with the following layout:
/// `[ kind: 4 bits | data: 28 bits ]`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AliasRegionKey {
    /// A `VMContext` field access.
    VMContext {
        /// The offset of the `VMContext` field being accessed (or the base
        /// of the array for `VMContext` array fields).
        offset: u32,
    },

    /// A `VMStoreContext` field access.
    VMStoreContext {
        /// The offset of the `VMStoreContext` field being accessed.
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
    #[allow(
        dead_code,
        reason = "easier not to cfg off; exact feature set is wonky in workspace"
    )]
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

    /// Encode this key into a raw `u32` suitable for use as an
    /// `AliasRegionData::user_id`.
    pub(crate) fn into_raw(self) -> u32 {
        match self {
            AliasRegionKey::VMContext { offset } => {
                debug_assert_eq!(offset & Self::KIND_MASK, 0);
                Self::VM_CONTEXT_KIND | (offset & Self::OFFSET_MASK)
            }
            AliasRegionKey::VMStoreContext { offset } => {
                debug_assert_eq!(offset & Self::KIND_MASK, 0);
                Self::VM_STORE_CONTEXT_KIND | (offset & Self::OFFSET_MASK)
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
            AliasRegionKey::VMContext { offset } => write!(f, "VMContext+{offset:#x}"),
            AliasRegionKey::VMStoreContext { offset } => write!(f, "VMStoreContext+{offset:#x}"),
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
