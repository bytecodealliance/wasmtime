use super::CallThreadState;
use crate::prelude::*;
use crate::runtime::vm::{Backtrace, VMRuntimeLimits};
use wasm_encoder::CoreDumpValue;

/// A WebAssembly Coredump
#[derive(Debug)]
pub struct CoreDumpStack {
    /// The backtrace containing the stack frames for the CoreDump
    pub bt: Backtrace,

    /// The locals for each frame in the backtrace.
    ///
    /// This is not currently implemented.
    #[allow(dead_code)]
    pub locals: Vec<Vec<CoreDumpValue>>,

    /// The operands for each stack frame
    ///
    /// This is not currently implemented.
    #[allow(dead_code)]
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
        let bt =
            unsafe { Backtrace::new_with_trap_state(limits, self.unwinder, self, trap_pc_and_fp) };

        Some(CoreDumpStack {
            bt,
            locals: vec![],
            operand_stack: vec![],
        })
    }
}
