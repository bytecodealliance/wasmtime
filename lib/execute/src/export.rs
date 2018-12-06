use cranelift_codegen::ir;
use cranelift_wasm::Global;
use vmcontext::{VMGlobal, VMMemory, VMTable};
use wasmtime_environ::{MemoryPlan, TablePlan};

/// The value of an export passed from one instance to another.
pub enum ExportValue {
    /// A function export value.
    Function {
        /// The address of the native-code function.
        address: usize,
        /// The function signature declaration, used for compatibilty checking.
        signature: ir::Signature,
    },

    /// A table export value.
    Table {
        /// The address of the table descriptor.
        address: *mut VMTable,
        /// The table declaration, used for compatibilty checking.
        table: TablePlan,
    },

    /// A memory export value.
    Memory {
        /// The address of the memory descriptor.
        address: *mut VMMemory,
        /// The memory declaration, used for compatibilty checking.
        memory: MemoryPlan,
    },

    /// A global export value.
    Global {
        /// The address of the global storage.
        address: *mut VMGlobal,
        /// The global declaration, used for compatibilty checking.
        global: Global,
    },
}

impl ExportValue {
    /// Construct a function export value.
    pub fn function(address: usize, signature: ir::Signature) -> Self {
        ExportValue::Function { address, signature }
    }

    /// Construct a table export value.
    pub fn table(address: *mut VMTable, table: TablePlan) -> Self {
        ExportValue::Table { address, table }
    }

    /// Construct a memory export value.
    pub fn memory(address: *mut VMMemory, memory: MemoryPlan) -> Self {
        ExportValue::Memory { address, memory }
    }

    /// Construct a global export value.
    pub fn global(address: *mut VMGlobal, global: Global) -> Self {
        ExportValue::Global { address, global }
    }
}

/// Import resolver connects imports with available exported values.
pub trait Resolver {
    /// Resolve the given module/field combo.
    fn resolve(&mut self, module: &str, field: &str) -> Option<ExportValue>;
}

/// `Resolver` implementation that always resolves to `None`.
pub struct NullResolver {}

impl Resolver for NullResolver {
    fn resolve(&mut self, _module: &str, _field: &str) -> Option<ExportValue> {
        None
    }
}
