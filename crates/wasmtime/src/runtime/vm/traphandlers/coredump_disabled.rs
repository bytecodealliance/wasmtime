use crate::runtime::vm::traphandlers::CallThreadState;
use crate::runtime::vm::VMStoreContext;

/// A WebAssembly Coredump
#[derive(Debug)]
pub enum CoreDumpStack {}

impl CallThreadState {
    pub(super) fn capture_coredump(
        &self,
        _ctx: *const VMStoreContext,
        _trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Option<CoreDumpStack> {
        None
    }
}
