use cranelift_codegen::ir;
use cranelift_wasm::Global;
use wasmtime_environ::{MemoryPlan, TablePlan};
use wasmtime_runtime::{
    VMContext, VMFunctionBody, VMGlobalDefinition, VMMemoryDefinition, VMTableDefinition,
};

/// An exported function.
pub struct FunctionExport {
    /// The address of the native-code function.
    pub address: *const VMFunctionBody,
    /// The function signature declaration, used for compatibilty checking.
    pub signature: ir::Signature,
}

/// The value of an export passed from one instance to another.
pub enum Export {
    /// A function export value.
    Function(FunctionExport),

    /// A table export value.
    Table {
        /// The address of the table descriptor.
        address: *mut VMTableDefinition,
        /// Pointer to the containing VMContext.
        vmctx: *mut VMContext,
        /// The table declaration, used for compatibilty checking.
        table: TablePlan,
    },

    /// A memory export value.
    Memory {
        /// The address of the memory descriptor.
        address: *mut VMMemoryDefinition,
        /// Pointer to the containing VMContext.
        vmctx: *mut VMContext,
        /// The memory declaration, used for compatibilty checking.
        memory: MemoryPlan,
    },

    /// A global export value.
    Global {
        /// The address of the global storage.
        address: *mut VMGlobalDefinition,
        /// The global declaration, used for compatibilty checking.
        global: Global,
    },
}

impl Export {
    /// Construct a function export value.
    pub fn function(address: *const VMFunctionBody, signature: ir::Signature) -> Self {
        Export::Function(FunctionExport { address, signature })
    }

    /// Construct a table export value.
    pub fn table(address: *mut VMTableDefinition, vmctx: *mut VMContext, table: TablePlan) -> Self {
        Export::Table {
            address,
            vmctx,
            table,
        }
    }

    /// Construct a memory export value.
    pub fn memory(
        address: *mut VMMemoryDefinition,
        vmctx: *mut VMContext,
        memory: MemoryPlan,
    ) -> Self {
        Export::Memory {
            address,
            vmctx,
            memory,
        }
    }

    /// Construct a global export value.
    pub fn global(address: *mut VMGlobalDefinition, global: Global) -> Self {
        Export::Global { address, global }
    }
}

/// Import resolver connects imports with available exported values.
pub trait Resolver {
    /// Resolve the given module/field combo.
    fn resolve(&mut self, module: &str, field: &str) -> Option<Export>;
}

/// `Resolver` implementation that always resolves to `None`.
pub struct NullResolver {}

impl Resolver for NullResolver {
    fn resolve(&mut self, _module: &str, _field: &str) -> Option<Export> {
        None
    }
}
