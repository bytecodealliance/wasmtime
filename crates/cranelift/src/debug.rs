//! Debug utils for WebAssembly using Cranelift.

/// Memory definition offset in the VMContext structure.
#[derive(Debug, Clone)]
pub enum ModuleMemoryOffset {
    /// Not available.
    None,
    /// Offset to the defined memory.
    Defined(u32),
    /// This memory is imported.
    Imported {
        /// Offset, in bytes, to the `*mut VMMemoryDefinition` structure within
        /// `VMContext`.
        offset_to_vm_memory_definition: u32,
        /// Offset, in bytes within `VMMemoryDefinition` where the `base` field
        /// lies.
        offset_to_memory_base: u32,
    },
}

pub use write_debuginfo::{emit_dwarf, DwarfSectionRelocTarget};

mod gc;
mod transform;
mod write_debuginfo;
