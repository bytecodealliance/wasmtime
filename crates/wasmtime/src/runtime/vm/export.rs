#[cfg(feature = "component-model")]
use crate::runtime::vm::component::VMComponentContext;
use crate::runtime::vm::vmcontext::{
    VMContext, VMFuncRef, VMGlobalDefinition, VMGlobalImport, VMGlobalKind, VMMemoryDefinition,
    VMOpaqueContext, VMTableDefinition, VMTagDefinition,
};
use core::ptr::NonNull;
#[cfg(feature = "component-model")]
use wasmtime_environ::component::RuntimeComponentInstanceIndex;
use wasmtime_environ::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, DefinedTagIndex, Global, Memory,
    Table, Tag,
};

/// The value of an export passed from one instance to another.
pub enum Export {
    /// A function export value.
    Function(ExportFunction),

    /// A table export value.
    Table(ExportTable),

    /// A memory export value.
    Memory(ExportMemory),

    /// A global export value.
    Global(ExportGlobal),

    /// A tag export value.
    Tag(ExportTag),
}

/// A function export value.
#[derive(Debug, Clone, Copy)]
pub struct ExportFunction {
    /// The `VMFuncRef` for this exported function.
    ///
    /// Note that exported functions cannot be a null funcref, so this is a
    /// non-null pointer.
    pub func_ref: NonNull<VMFuncRef>,
}

// As part of the contract for using `ExportFunction`, synchronization
// properties must be upheld. Therefore, despite containing raw pointers,
// it is declared as Send/Sync.
unsafe impl Send for ExportFunction {}
unsafe impl Sync for ExportFunction {}

impl From<ExportFunction> for Export {
    fn from(func: ExportFunction) -> Export {
        Export::Function(func)
    }
}

/// A table export value.
#[derive(Debug, Clone)]
pub struct ExportTable {
    /// The address of the table descriptor.
    pub definition: NonNull<VMTableDefinition>,
    /// Pointer to the containing `VMContext`.
    pub vmctx: NonNull<VMContext>,
    /// The table declaration, used for compatibility checking.
    pub table: Table,
    /// The index at which the table is defined within the `vmctx`.
    pub index: DefinedTableIndex,
}

// See docs on send/sync for `ExportFunction` above.
unsafe impl Send for ExportTable {}
unsafe impl Sync for ExportTable {}

impl From<ExportTable> for Export {
    fn from(func: ExportTable) -> Export {
        Export::Table(func)
    }
}

/// A memory export value.
#[derive(Debug, Clone)]
pub struct ExportMemory {
    /// The address of the memory descriptor.
    pub definition: NonNull<VMMemoryDefinition>,
    /// Pointer to the containing `VMContext`.
    pub vmctx: NonNull<VMContext>,
    /// The memory declaration, used for compatibility checking.
    pub memory: Memory,
    /// The index at which the memory is defined within the `vmctx`.
    pub index: DefinedMemoryIndex,
}

// See docs on send/sync for `ExportFunction` above.
unsafe impl Send for ExportMemory {}
unsafe impl Sync for ExportMemory {}

impl From<ExportMemory> for Export {
    fn from(func: ExportMemory) -> Export {
        Export::Memory(func)
    }
}

/// A global export value.
#[derive(Debug, Clone)]
pub struct ExportGlobal {
    /// The address of the global storage.
    pub definition: NonNull<VMGlobalDefinition>,

    /// Kind of exported global, or what's owning this global and how to find
    /// it.
    pub kind: ExportGlobalKind,

    /// The global declaration, used for compatibility checking.
    pub global: Global,
}

/// A global export value.
#[derive(Debug, Clone)]
pub enum ExportGlobalKind {
    /// This global was created by the host or embedder and is stored within the
    /// `Store` at the provided offset.
    Host(DefinedGlobalIndex),

    /// This global was created as part of a core wasm instance.
    Instance(NonNull<VMContext>, DefinedGlobalIndex),

    /// This global was created as flags for a component instance.
    #[cfg(feature = "component-model")]
    ComponentFlags(NonNull<VMComponentContext>, RuntimeComponentInstanceIndex),
}

impl ExportGlobal {
    pub fn from_vmimport(import: &VMGlobalImport, ty: Global) -> ExportGlobal {
        let kind = match import.kind {
            VMGlobalKind::Host(index) => ExportGlobalKind::Host(index),
            VMGlobalKind::Instance(index) => ExportGlobalKind::Instance(
                unsafe { VMContext::from_opaque(import.vmctx.unwrap().as_non_null()) },
                index,
            ),
            #[cfg(feature = "component-model")]
            VMGlobalKind::ComponentFlags(index) => ExportGlobalKind::ComponentFlags(
                unsafe { VMComponentContext::from_opaque(import.vmctx.unwrap().as_non_null()) },
                index,
            ),
        };
        ExportGlobal {
            definition: import.from.as_non_null(),
            kind,
            global: ty,
        }
    }

    pub fn vmimport(&self) -> VMGlobalImport {
        let (vmctx, kind) = match self.kind {
            ExportGlobalKind::Host(index) => (None, VMGlobalKind::Host(index)),
            ExportGlobalKind::Instance(vmctx, index) => (
                Some(VMOpaqueContext::from_vmcontext(vmctx).into()),
                VMGlobalKind::Instance(index),
            ),
            #[cfg(feature = "component-model")]
            ExportGlobalKind::ComponentFlags(vmctx, index) => (
                Some(VMOpaqueContext::from_vmcomponent(vmctx).into()),
                VMGlobalKind::ComponentFlags(index),
            ),
        };
        VMGlobalImport {
            from: self.definition.into(),
            vmctx,
            kind,
        }
    }
}

// See docs on send/sync for `ExportFunction` above.
unsafe impl Send for ExportGlobal {}
unsafe impl Sync for ExportGlobal {}

impl From<ExportGlobal> for Export {
    fn from(func: ExportGlobal) -> Export {
        Export::Global(func)
    }
}

/// A tag export value.
#[derive(Debug, Clone)]
pub struct ExportTag {
    /// The address of the global storage.
    pub definition: NonNull<VMTagDefinition>,
    /// The instance that owns this tag.
    pub vmctx: NonNull<VMContext>,
    /// The global declaration, used for compatibility checking.
    pub tag: Tag,
    /// The index at which the tag is defined within the `vmctx`.
    pub index: DefinedTagIndex,
}

// See docs on send/sync for `ExportFunction` above.
unsafe impl Send for ExportTag {}
unsafe impl Sync for ExportTag {}

impl From<ExportTag> for Export {
    fn from(func: ExportTag) -> Export {
        Export::Tag(func)
    }
}
