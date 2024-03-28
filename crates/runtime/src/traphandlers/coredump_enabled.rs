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

impl CallThreadState {
    pub(super) fn capture_coredump(
        &self,
        limits: *const VMRuntimeLimits,
        trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Option<CoreDumpStack> {
        if !self.capture_coredump {
            return None;
        }
        let bt = unsafe { Backtrace::new_with_trap_state(limits, self, trap_pc_and_fp) };

        Some(CoreDumpStack {
            bt,
            locals: vec![],
            operand_stack: vec![],
        })
    }
}
