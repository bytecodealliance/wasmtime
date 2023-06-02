use wasm_encoder::CoreDumpValue;

use crate::{Backtrace, VMRuntimeLimits};

use super::CallThreadState;

/// A WebAssembly Coredump
#[derive(Debug)]
pub struct CoreDumpStack {
    /// The backtrace containing the stack frames for the CoreDump
    pub bt: Backtrace,

    /// Unimplemented
    /// The indices of the locals and operand_stack all map to each other (ie.
    /// index 0 is the locals for the first frame in the backtrace, etc)
    pub locals: Vec<Vec<CoreDumpValue>>,

    /// Unimplemented
    /// The operands for each stack frame
    pub operand_stack: Vec<Vec<CoreDumpValue>>,
}

impl CoreDumpStack {
    /// Capture a core dump of the current wasm state
    pub fn new(
        cts: &CallThreadState,
        limits: *const VMRuntimeLimits,
        trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Self {
        let bt = unsafe { Backtrace::new_with_trap_state(limits, cts, trap_pc_and_fp) };

        Self {
            bt,
            locals: vec![],
            operand_stack: vec![],
        }
    }
}
