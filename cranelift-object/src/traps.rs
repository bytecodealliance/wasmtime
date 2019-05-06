//! Records every `TrapCode` that cranelift outputs during code generation,
//! for every function in the module. This data may be useful at runtime.

use cranelift_codegen::{binemit, ir};

/// Record of the arguments cranelift passes to `TrapSink::trap`
#[derive(Clone)]
pub struct ObjectTrapSite {
    /// Offset into function
    pub offset: binemit::CodeOffset,
    /// Source location given to cranelift
    pub srcloc: ir::SourceLoc,
    /// Trap code, as determined by cranelift
    pub code: ir::TrapCode,
}

/// Record of the trap sites for a given function
#[derive(Default, Clone)]
pub struct ObjectTrapSink {
    /// All trap sites collected in function
    pub sites: Vec<ObjectTrapSite>,
}

impl binemit::TrapSink for ObjectTrapSink {
    fn trap(&mut self, offset: binemit::CodeOffset, srcloc: ir::SourceLoc, code: ir::TrapCode) {
        self.sites.push(ObjectTrapSite {
            offset,
            srcloc,
            code,
        });
    }
}
