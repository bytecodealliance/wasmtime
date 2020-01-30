use wasmtime_environ::ir::{SourceLoc, TrapCode};
use wasmtime_environ::TrapInformation;
use wasmtime_jit::trampoline::binemit;

pub(crate) struct TrapSink {
    pub traps: Vec<TrapInformation>,
}

impl TrapSink {
    pub fn new() -> Self {
        Self { traps: Vec::new() }
    }
}

impl binemit::TrapSink for TrapSink {
    fn trap(
        &mut self,
        code_offset: binemit::CodeOffset,
        source_loc: SourceLoc,
        trap_code: TrapCode,
    ) {
        self.traps.push(TrapInformation {
            code_offset,
            source_loc,
            trap_code,
        });
    }
}
