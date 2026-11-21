use super::*;
use cranelift_control::ControlPlane;

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// Tracks user stack maps for safepoint instructions.
    pub(crate) stack_maps: StackMapEmitter<Inst>,

    /// Only used during fuzz-testing. Otherwise, it is a zero-sized struct and
    /// optimized away at compiletime. See [cranelift_control].
    ctrl_plane: ControlPlane,

    /// A copy of the frame layout, used during the emission of `Inst::ReturnCallKnown` and
    /// `Inst::ReturnCallUnknown` instructions and exception callsites.
    pub frame_layout: FrameLayout,

    /// Offset at which post-call debug tags for the call currently being
    /// emitted should be placed (i.e., the call's return address).
    pub(crate) post_call_debug_tag_offset: Option<CodeOffset>,
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &Callee<X64ABIMachineSpec>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            stack_maps: StackMapEmitter::default(),
            ctrl_plane,
            frame_layout: abi.frame_layout().clone(),
            post_call_debug_tag_offset: None,
        }
    }

    fn stack_map_emitter(&mut self) -> &mut StackMapEmitter<Inst> {
        &mut self.stack_maps
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

    fn post_call_debug_tag_offset_mut(&mut self) -> &mut Option<CodeOffset> {
        &mut self.post_call_debug_tag_offset
    }
}
