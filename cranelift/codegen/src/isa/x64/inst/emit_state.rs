use super::*;
use crate::ir;
use cranelift_control::ControlPlane;

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// The user stack map for the upcoming instruction, as provided to
    /// `pre_safepoint()`.
    user_stack_map: Option<ir::UserStackMap>,

    /// Last emitted map's PC; a following faulting poll needs a distinct PC.
    last_stack_map_offset: Option<u32>,

    /// Only used during fuzz-testing. Otherwise, it is a zero-sized struct and
    /// optimized away at compiletime. See [cranelift_control].
    ctrl_plane: ControlPlane,

    /// A copy of the frame layout, used during the emission of `Inst::ReturnCallKnown` and
    /// `Inst::ReturnCallUnknown` instructions and exception callsites.
    pub frame_layout: FrameLayout,
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &Callee<X64ABIMachineSpec>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            user_stack_map: None,
            last_stack_map_offset: None,
            ctrl_plane,
            frame_layout: abi.frame_layout().clone(),
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
    pub(crate) fn push_stack_map(
        &mut self,
        sink: &mut MachBuffer<Inst>,
        offset: u32,
        map: ir::UserStackMap,
    ) {
        sink.push_user_stack_map(self, offset, map);
        self.last_stack_map_offset = Some(offset);
    }

    pub(crate) fn pad_for_stack_map(&self, sink: &mut MachBuffer<Inst>) {
        if self.last_stack_map_offset == Some(sink.cur_offset()) {
            sink.put1(0x90); // x86 NOP
        }
    }

    pub(crate) fn take_stack_map(&mut self) -> Option<ir::UserStackMap> {
        self.user_stack_map.take()
    }

    pub(crate) fn clear_post_insn(&mut self) {
        self.user_stack_map = None;
    }
}
