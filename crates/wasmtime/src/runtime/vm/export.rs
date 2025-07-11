use crate::runtime::vm::vmcontext::{VMContext, VMFuncRef, VMMemoryDefinition};
use core::ptr::NonNull;
use wasmtime_environ::{DefinedMemoryIndex, Memory};

/// The value of an export passed from one instance to another.
pub enum Export {
    /// A function export value.
    Function(ExportFunction),

    /// A table export value.
    Table(crate::Table),

    /// A memory export value.
    Memory(ExportMemory),

    /// A global export value.
    Global(crate::Global),

    /// A tag export value.
    Tag(crate::Tag),
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
