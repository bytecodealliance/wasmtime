use crate::runtime::vm::traphandlers::CallThreadState;
use crate::runtime::vm::VMRuntimeLimits;

/// A WebAssembly Coredump
#[derive(Debug)]
pub enum CoreDumpStack {}

impl CallThreadState {
    pub(super) fn capture_coredump(
        &self,
        _limits: *const VMRuntimeLimits,
        _trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Option<CoreDumpStack> {
        None
    }
}
