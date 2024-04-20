//! Debug utils for WebAssembly using Cranelift.

/// Memory definition offset in the VMContext structure.
#[derive(Debug, Clone)]
pub enum ModuleMemoryOffset {
    /// Not available.
    None,
    /// Offset to the defined memory.
    Defined(u32),
    /// Offset to the imported memory.
    Imported(#[allow(dead_code)] u32),
}

pub use write_debuginfo::{emit_dwarf, DwarfSectionRelocTarget};

mod gc;
mod transform;
mod write_debuginfo;
