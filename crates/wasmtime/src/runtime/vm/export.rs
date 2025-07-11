use crate::runtime::vm::vmcontext::VMFuncRef;
use core::ptr::NonNull;

/// The value of an export passed from one instance to another.
pub enum Export {
    /// A function export value.
    Function(ExportFunction),

    /// A table export value.
    Table(crate::Table),

    /// A memory export value.
    Memory { memory: crate::Memory, shared: bool },

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
