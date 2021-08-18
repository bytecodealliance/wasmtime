//! Data structures to provide transformation of the source
// addresses of a WebAssembly module into the native code.

use serde::{Deserialize, Serialize};

/// Single source location to generated address mapping.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstructionAddressMap {
    /// Where in the source wasm binary this instruction comes from, specified
    /// in an offset of bytes from the front of the file.
    pub srcloc: FilePos,

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

    /// Function's initial offset in the source file, specified in bytes from
    /// the front of the file.
    pub start_srcloc: FilePos,

    /// Function's end offset in the source file, specified in bytes from
    /// the front of the file.
    pub end_srcloc: FilePos,

    /// Generated function body offset if applicable, otherwise 0.
    pub body_offset: usize,

    /// Generated function body length.
    pub body_len: u32,
}

/// A position within an original source file,
///
/// This structure is used as a newtype wrapper around a 32-bit integer which
/// represents an offset within a file where a wasm instruction or function is
/// to be originally found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePos(u32);

impl FilePos {
    /// Create a new file position with the given offset.
    pub fn new(pos: u32) -> FilePos {
        assert!(pos != u32::MAX);
        FilePos(pos)
    }

    /// Returns the offset that this offset was created with.
    ///
    /// Note that the `Default` implementation will return `None` here, whereas
    /// positions created with `FilePos::new` will return `Some`.
    pub fn file_offset(self) -> Option<u32> {
        if self.0 == u32::MAX {
            None
        } else {
            Some(self.0)
        }
    }
}

impl Default for FilePos {
    fn default() -> FilePos {
        FilePos(u32::MAX)
    }
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
