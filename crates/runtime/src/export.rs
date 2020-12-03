use crate::vmcontext::{
    VMCallerCheckedAnyfunc, VMContext, VMGlobalDefinition, VMMemoryDefinition, VMTableDefinition,
};
use crate::InstanceHandle;
use std::any::Any;
use std::ptr::NonNull;
use wasmtime_environ::wasm::Global;
use wasmtime_environ::{MemoryPlan, TablePlan};

/// The value of an export passed from one instance to another.
pub enum Export<'a> {
    /// A function export value.
    Function(ExportFunction),

    /// A table export value.
    Table(ExportTable),

    /// A memory export value.
    Memory(ExportMemory),

    /// A global export value.
    Global(ExportGlobal),

    /// An instance
    Instance(&'a InstanceHandle),

    /// A module
    Module(&'a dyn Any),
}

/// A function export value.
#[derive(Debug, Clone)]
pub struct ExportFunction {
    /// The `VMCallerCheckedAnyfunc` for this exported function.
    ///
    /// Note that exported functions cannot be a null funcref, so this is a
    /// non-null pointer.
    pub anyfunc: NonNull<VMCallerCheckedAnyfunc>,
}

impl<'a> From<ExportFunction> for Export<'a> {
    fn from(func: ExportFunction) -> Export<'a> {
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

impl<'a> From<ExportTable> for Export<'a> {
    fn from(func: ExportTable) -> Export<'a> {
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

impl<'a> From<ExportMemory> for Export<'a> {
    fn from(func: ExportMemory) -> Export<'a> {
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

impl<'a> From<ExportGlobal> for Export<'a> {
    fn from(func: ExportGlobal) -> Export<'a> {
        Export::Global(func)
    }
}
