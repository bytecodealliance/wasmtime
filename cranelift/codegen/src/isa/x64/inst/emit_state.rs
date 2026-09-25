use super::*;
use crate::ir;
use cranelift_control::ControlPlane;

/// State carried between emissions of a sequence of instructions.
#[derive(Clone, Debug)]
pub struct EmitState {
    /// The user stack map for the upcoming instruction, as provided to
    /// `pre_safepoint()`.
    user_stack_map: Option<ir::UserStackMap>,

    /// Only used during fuzz-testing. Otherwise, it is a zero-sized struct and
    /// optimized away at compiletime. See [cranelift_control].
    ctrl_plane: ControlPlane,

    /// A copy of the frame layout, used during the emission of `Inst::ReturnCallKnown` and
    /// `Inst::ReturnCallUnknown` instructions and exception callsites.
    pub frame_layout: FrameLayout,

    /// The function's calling convention, used for return-call epilogue
    /// teardown (e.g. FP-less GHC frames).
    pub call_conv: CallConv,
}

impl Default for EmitState {
    fn default() -> Self {
        EmitState {
            user_stack_map: None,
            ctrl_plane: ControlPlane::default(),
            frame_layout: FrameLayout::default(),
            call_conv: CallConv::SystemV,
        }
    }
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &Callee<X64ABIMachineSpec>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            user_stack_map: None,
            ctrl_plane,
            frame_layout: abi.frame_layout().clone(),
            call_conv: abi.call_conv(),
        }
    }

    fn pre_safepoint(&mut self, user_stack_map: Option<ir::UserStackMap>) {
        self.user_stack_map = user_stack_map;
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
    pub(crate) fn take_stack_map(&mut self) -> Option<ir::UserStackMap> {
        self.user_stack_map.take()
    }

    pub(crate) fn clear_post_insn(&mut self) {
        self.user_stack_map = None;
    }
}
