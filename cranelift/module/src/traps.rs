//! Defines `TrapSite`.

use cranelift_codegen::{binemit, ir};

/// Record of the arguments cranelift passes to `TrapSink::trap`.
#[derive(Clone, Debug)]
pub struct TrapSite {
    /// Offset into function.
    pub offset: binemit::CodeOffset,
    /// Source location given to cranelift.
    pub srcloc: ir::SourceLoc,
    /// Trap code, as determined by cranelift.
    pub code: ir::TrapCode,
}
