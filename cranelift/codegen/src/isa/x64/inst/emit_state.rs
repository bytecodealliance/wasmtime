use super::*;
use cranelift_control::ControlPlane;

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// Offset of FP from nominal-SP.
    nominal_sp_to_fp: i64,
    /// Safepoint stack map for upcoming instruction, as provided to `pre_safepoint()`.
    stack_map: Option<StackMap>,
    /// Only used during fuzz-testing. Otherwise, it is a zero-sized struct and
    /// optimized away at compiletime. See [cranelift_control].
    ctrl_plane: ControlPlane,

    /// A copy of the frame layout, used during the emission of `Inst::ReturnCallKnown` and
    /// `Inst::ReturnCallUnknown` instructions.
    frame_layout: FrameLayout,
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &Callee<X64ABIMachineSpec>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            nominal_sp_to_fp: abi.frame_size() as i64,
            stack_map: None,
            ctrl_plane,
            frame_layout: abi.frame_layout().clone(),
        }
    }

    fn pre_safepoint(&mut self, stack_map: StackMap) {
        self.stack_map = Some(stack_map);
    }

    fn ctrl_plane_mut(&mut self) -> &mut ControlPlane {
        &mut self.ctrl_plane
    }

    fn take_ctrl_plane(self) -> ControlPlane {
        self.ctrl_plane
    }

    fn frame_layout(&self) -> &FrameLayout {
        &self.frame_layout
    }
}

impl EmitState {
    pub(crate) fn take_stack_map(&mut self) -> Option<StackMap> {
        self.stack_map.take()
    }

    pub(crate) fn clear_post_insn(&mut self) {
        self.stack_map = None;
    }

    pub(crate) fn nominal_sp_to_fp(&self) -> i64 {
        self.nominal_sp_to_fp
    }
}
