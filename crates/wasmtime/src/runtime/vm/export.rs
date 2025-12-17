use crate::runtime::vm::{SharedMemory, VMMemoryImport};

/// The value of an export passed from one instance to another.
pub enum Export {
    /// A function export value.
    Function(crate::Func),

    /// A table export value.
    Table(crate::Table),

    /// An unshared memory export value.
    Memory(crate::Memory),

    /// A shared memory export value.
    SharedMemory(SharedMemory, VMMemoryImport),

    /// A global export value.
    Global(crate::Global),

    /// A tag export value.
    Tag(crate::Tag),
}

pub enum ExportMemory {
    Unshared(crate::Memory),
    Shared(SharedMemory, VMMemoryImport),
}

impl ExportMemory {
    pub fn unshared(self) -> Option<crate::Memory> {
        match self {
            ExportMemory::Unshared(m) => Some(m),
            ExportMemory::Shared(..) => None,
        }
    }
}
