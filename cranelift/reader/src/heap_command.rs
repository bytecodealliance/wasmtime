//! Heap commands.
//!
//! Functions in a `.clif` file can have *heap commands* appended that control the heaps allocated
//! by the `test run` and `test interpret` infrastructure.
//!
//! The general syntax is:
//! - `; heap: <heap_type>, size=n`
//!
//! `heap_type` can have two values:
//! - `static`: This is a non resizable heap type with a fixed size
//! - `dynamic`: This is a resizable heap, which can grow
//!
//! `size=n` indicates the size of the heap. For dynamic heaps, it indicates the starting size of
//! the heap.

use cranelift_codegen::ir::immediates::Uimm64;
use std::fmt::{self, Display, Formatter};

/// A heap command appearing in a test file.
///
/// For parsing, see `Parser::parse_heap_command`
#[derive(PartialEq, Debug, Clone)]
pub struct HeapCommand {
    /// Indicates the requested heap type
    pub heap_type: HeapType,
    /// Size of the heap.
    ///
    /// For dynamic heaps this is the starting size. For static heaps, this is the total size.
    pub size: Uimm64,
    /// Offset of the heap pointer from the vmctx base
    ///
    /// This is done for verification purposes only
    pub ptr_offset: Option<Uimm64>,
    /// Offset of the bound pointer from the vmctx base
    ///
    /// This is done for verification purposes only
    pub bound_offset: Option<Uimm64>,
}

impl Display for HeapCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "heap: {}, size={}", self.heap_type, self.size)?;

        if let Some(offset) = self.ptr_offset {
            write!(f, ", ptr=vmctx+{}", offset)?
        }

        if let Some(offset) = self.bound_offset {
            write!(f, ", bound=vmctx+{}", offset)?
        }

        Ok(())
    }
}

/// CLIF Representation of a heap type. e.g.: `static`
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Clone)]
pub enum HeapType {
    Static,
    Dynamic,
}

impl Display for HeapType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HeapType::Static => write!(f, "static"),
            HeapType::Dynamic => write!(f, "dynamic"),
        }
    }
}
