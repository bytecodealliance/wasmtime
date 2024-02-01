use super::*;
use cranelift_control::ControlPlane;

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// Addend to convert nominal-SP offsets to real-SP offsets at the current
    /// program point.
    virtual_sp_offset: i64,
    /// Offset of FP from nominal-SP.
    nominal_sp_to_fp: i64,
    /// Safepoint stack map for upcoming instruction, as provided to `pre_safepoint()`.
    stack_map: Option<StackMap>,
    /// Current source location.
    cur_srcloc: RelSourceLoc,
    /// Only used during fuzz-testing. Otherwise, it is a zero-sized struct and
    /// optimized away at compiletime. See [cranelift_control].
    ctrl_plane: ControlPlane,
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &Callee<X64ABIMachineSpec>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            virtual_sp_offset: 0,
            nominal_sp_to_fp: abi.frame_size() as i64,
            stack_map: None,
            cur_srcloc: Default::default(),
            ctrl_plane,
        }
    }

    fn pre_safepoint(&mut self, stack_map: StackMap) {
        self.stack_map = Some(stack_map);
    }

    fn pre_sourceloc(&mut self, srcloc: RelSourceLoc) {
        self.cur_srcloc = srcloc;
    }

    fn ctrl_plane_mut(&mut self) -> &mut ControlPlane {
        &mut self.ctrl_plane
    }

    fn take_ctrl_plane(self) -> ControlPlane {
        self.ctrl_plane
    }
}

impl EmitState {
    pub(crate) fn take_stack_map(&mut self) -> Option<StackMap> {
        self.stack_map.take()
    }

    pub(crate) fn clear_post_insn(&mut self) {
        self.stack_map = None;
    }

    pub(crate) fn virtual_sp_offset(&self) -> i64 {
        self.virtual_sp_offset
    }

    pub(crate) fn adjust_virtual_sp_offset(&mut self, amount: i64) {
        let old = self.virtual_sp_offset;
        let new = self.virtual_sp_offset + amount;
        trace!("adjust virtual sp offset by {amount:#x}: {old:#x} -> {new:#x}",);
        self.virtual_sp_offset = new;
    }

    pub(crate) fn nominal_sp_to_fp(&self) -> i64 {
        self.nominal_sp_to_fp
    }
}
