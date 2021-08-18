//! Debug utils for WebAssembly using Cranelift.

#![allow(clippy::cast_ptr_alignment)]

pub use write_debuginfo::{emit_dwarf, DwarfSection, DwarfSectionRelocTarget};

mod gc;
mod transform;
mod write_debuginfo;
