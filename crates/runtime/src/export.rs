use crate::vmcontext::{
    VMContext, VMFunctionBody, VMGlobalDefinition, VMMemoryDefinition, VMSharedSignatureIndex,
    VMTableDefinition,
};
use wasmtime_environ::wasm::Global;
use wasmtime_environ::{MemoryPlan, TablePlan};

/// The value of an export passed from one instance to another.
#[derive(Debug, Clone)]
pub enum Export {
    /// A function export value.
    Function(ExportFunction),

    /// A table export value.
    Table(ExportTable),

    /// A memory export value.
    Memory(ExportMemory),

    /// A global export value.
    Global(ExportGlobal),
}

/// A function export value.
#[derive(Debug, Clone)]
pub struct ExportFunction {
    /// The address of the native-code function.
    pub address: *const VMFunctionBody,
    /// Pointer to the containing `VMContext`.
    pub vmctx: *mut VMContext,
    /// The function signature declaration, used for compatibilty checking.
    ///
    /// Note that this indexes within the module associated with `vmctx`.
    pub signature: VMSharedSignatureIndex,
}

impl From<ExportFunction> for Export {
    fn from(func: ExportFunction) -> Export {
        Export::Function(func)
    }
}

/// A table export value.
#[derive(Debug, Clone)]
pub struct ExportTable {
    /// The address of the table descriptor.
    pub definition: *mut VMTableDefinition,
    /// Pointer to the containing `VMContext`.
    pub vmctx: *mut VMContext,
    /// The table declaration, used for compatibilty checking.
    pub table: TablePlan,
}

impl From<ExportTable> for Export {
    fn from(func: ExportTable) -> Export {
        Export::Table(func)
    }
}

/// A memory export value.
#[derive(Debug, Clone)]
pub struct ExportMemory {
    /// The address of the memory descriptor.
    pub definition: *mut VMMemoryDefinition,
    /// Pointer to the containing `VMContext`.
    pub vmctx: *mut VMContext,
    /// The memory declaration, used for compatibilty checking.
    pub memory: MemoryPlan,
}

impl From<ExportMemory> for Export {
    fn from(func: ExportMemory) -> Export {
        Export::Memory(func)
    }
}

/// A global export value.
#[derive(Debug, Clone)]
pub struct ExportGlobal {
    /// The address of the global storage.
    pub definition: *mut VMGlobalDefinition,
    /// Pointer to the containing `VMContext`.
    pub vmctx: *mut VMContext,
    /// The global declaration, used for compatibilty checking.
    pub global: Global,
}

impl From<ExportGlobal> for Export {
    fn from(func: ExportGlobal) -> Export {
        Export::Global(func)
    }
}
