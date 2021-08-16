//! Debug utils for WebAssembly using Cranelift.

#![allow(clippy::cast_ptr_alignment)]

pub use crate::write_debuginfo::{emit_dwarf, DwarfSection, DwarfSectionRelocTarget};

mod gc;
mod transform;
mod write_debuginfo;
