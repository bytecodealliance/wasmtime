//! Records every `TrapCode` that cranelift outputs during code generation,
//! for every function in the module. This data may be useful at runtime.

use cranelift_codegen::{binemit, ir};
use cranelift_module::TrapSite;

/// Record of the trap sites for a given function
#[derive(Default, Clone)]
pub struct ObjectTrapSink {
    /// All trap sites collected in function
    pub sites: Vec<TrapSite>,
}

impl binemit::TrapSink for ObjectTrapSink {
    fn trap(&mut self, offset: binemit::CodeOffset, srcloc: ir::SourceLoc, code: ir::TrapCode) {
        self.sites.push(TrapSite {
            offset,
            srcloc,
            code,
        });
    }
}
