//! Data structures to provide transformation of the source
// addresses of a WebAssembly module into the native code.

use cranelift_codegen::ir;
use serde::{Deserialize, Serialize};

/// Single source location to generated address mapping.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstructionAddressMap {
    /// Where in the source this instruction comes from.
    pub srcloc: ir::SourceLoc,

    /// Offset from the start of the function's compiled code to where this
    /// instruction is located, or the region where it starts.
    pub code_offset: u32,
}

/// Function and its instructions addresses mappings.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionAddressMap {
    /// An array of data for the instructions in this function, indicating where
    /// each instruction maps back to in the original function.
    ///
    /// This array is sorted least-to-greatest by the `code_offset` field.
    /// Additionally the span of each `InstructionAddressMap` is implicitly the
    /// gap between it and the next item in the array.
    pub instructions: Box<[InstructionAddressMap]>,

    /// Function start source location (normally declaration).
    pub start_srcloc: ir::SourceLoc,

    /// Function end source location.
    pub end_srcloc: ir::SourceLoc,

    /// Generated function body offset if applicable, otherwise 0.
    pub body_offset: usize,

    /// Generated function body length.
    pub body_len: u32,
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
