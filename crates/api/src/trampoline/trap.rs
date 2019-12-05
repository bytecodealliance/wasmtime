use std::cell::Cell;

use crate::r#ref::HostRef;
use crate::Trap;
use wasmtime_environ::ir::{SourceLoc, TrapCode};
use wasmtime_environ::TrapInformation;
use wasmtime_jit::trampoline::binemit;

// Randomly selected user TrapCode magic number 13.
pub const API_TRAP_CODE: TrapCode = TrapCode::User(13);

thread_local! {
    static RECORDED_API_TRAP: Cell<Option<HostRef<Trap>>> = Cell::new(None);
}

pub fn record_api_trap(trap: HostRef<Trap>) {
    RECORDED_API_TRAP.with(|data| {
        let trap = Cell::new(Some(trap));
        data.swap(&trap);
        assert!(
            trap.take().is_none(),
            "Only one API trap per thread can be recorded at a moment!"
        );
    });
}

pub fn take_api_trap() -> Option<HostRef<Trap>> {
    RECORDED_API_TRAP.with(|data| data.take())
}

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
