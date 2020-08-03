//! Data structures to provide transformation of the source
// addresses of a WebAssembly module into the native code.

use cranelift_codegen::ir;
use serde::{Deserialize, Serialize};

/// Single source location to generated address mapping.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstructionAddressMap {
    /// Original source location.
    pub srcloc: ir::SourceLoc,

    /// Generated instructions offset.
    pub code_offset: usize,

    /// Generated instructions length.
    pub code_len: usize,
}

/// Function and its instructions addresses mappings.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionAddressMap {
    /// Instructions maps.
    /// The array is sorted by the InstructionAddressMap::code_offset field.
    pub instructions: Vec<InstructionAddressMap>,

    /// Function start source location (normally declaration).
    pub start_srcloc: ir::SourceLoc,

    /// Function end source location.
    pub end_srcloc: ir::SourceLoc,

    /// Generated function body offset if applicable, otherwise 0.
    pub body_offset: usize,

    /// Generated function body length.
    pub body_len: usize,
}

/// Memory definition offset in the VMContext structure.
#[derive(Debug, Clone)]
pub enum ModuleMemoryOffset {
    /// Not available.
    None,
    /// Offset to the defined memory.
    Defined(u32),
    /// Offset to the imported memory.
    Imported(u32),
}
