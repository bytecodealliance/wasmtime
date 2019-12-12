use crate::vmcontext::{
    VMContext, VMFunctionBody, VMGlobalDefinition, VMMemoryDefinition, VMTableDefinition,
};
use wasmtime_environ::ir;
use wasmtime_environ::wasm::Global;
use wasmtime_environ::{MemoryPlan, TablePlan};

/// The value of an export passed from one instance to another.
#[derive(Debug, Clone)]
pub enum Export {
    /// A function export value.
    Function {
        /// The address of the native-code function.
        address: *const VMFunctionBody,
        /// Pointer to the containing `VMContext`.
        vmctx: *mut VMContext,
        /// The function signature declaration, used for compatibilty checking.
        signature: ir::Signature,
    },

    /// A table export value.
    Table {
        /// The address of the table descriptor.
        definition: *mut VMTableDefinition,
        /// Pointer to the containing `VMContext`.
        vmctx: *mut VMContext,
        /// The table declaration, used for compatibilty checking.
        table: TablePlan,
    },

    /// A memory export value.
    Memory {
        /// The address of the memory descriptor.
        definition: *mut VMMemoryDefinition,
        /// Pointer to the containing `VMContext`.
        vmctx: *mut VMContext,
        /// The memory declaration, used for compatibilty checking.
        memory: MemoryPlan,
    },

    /// A global export value.
    Global {
        /// The address of the global storage.
        definition: *mut VMGlobalDefinition,
        /// Pointer to the containing `VMContext`.
        vmctx: *mut VMContext,
        /// The global declaration, used for compatibilty checking.
        global: Global,
    },
}

impl Export {
    /// Construct a function export value.
    pub fn function(
        address: *const VMFunctionBody,
        vmctx: *mut VMContext,
        signature: ir::Signature,
    ) -> Self {
        Self::Function {
            address,
            vmctx,
            signature,
        }
    }

    /// Construct a table export value.
    pub fn table(
        definition: *mut VMTableDefinition,
        vmctx: *mut VMContext,
        table: TablePlan,
    ) -> Self {
        Self::Table {
            definition,
            vmctx,
            table,
        }
    }

    /// Construct a memory export value.
    pub fn memory(
        definition: *mut VMMemoryDefinition,
        vmctx: *mut VMContext,
        memory: MemoryPlan,
    ) -> Self {
        Self::Memory {
            definition,
            vmctx,
            memory,
        }
    }

    /// Construct a global export value.
    pub fn global(
        definition: *mut VMGlobalDefinition,
        vmctx: *mut VMContext,
        global: Global,
    ) -> Self {
        Self::Global {
            definition,
            vmctx,
            global,
        }
    }
}
