use cranelift_control::ControlPlane;

use super::Inst;
use crate::{
    FrameLayout, MachInstEmit, MachInstEmitState, ir, isa::arm32::abi::Arm32MachineDeps,
    machinst::Callee, settings,
};

pub struct EmitInfo {
    #[expect(dead_code, reason = "will be used in the future")]
    shared_flags: settings::Flags,
    isa_flags: crate::isa::arm32::settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(
        shared_flags: settings::Flags,
        isa_flags: crate::isa::arm32::settings::Flags,
    ) -> Self {
        EmitInfo {
            shared_flags,
            isa_flags,
        }
    }
}

/// Stub state carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// The user stack map for the upcoming instruction.
    _user_stack_map: Option<ir::UserStackMap>,
    /// Control plane (stub - not fully functional).
    ctrl_plane: ControlPlane,
    frame_layout: FrameLayout,
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(_abi: &Callee<Arm32MachineDeps>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            _user_stack_map: None,
            ctrl_plane,
            frame_layout: FrameLayout::default(),
        }
    }

    fn pre_safepoint(&mut self, user_stack_map: Option<ir::UserStackMap>) {
        todo!()
    }

    fn ctrl_plane_mut(&mut self) -> &mut ControlPlane {
        todo!()
    }

    fn take_ctrl_plane(self) -> ControlPlane {
        todo!()
    }

    fn frame_layout(&self) -> &FrameLayout {
        todo!()
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, code: &mut crate::MachBuffer<Self>, info: &Self::Info, state: &mut Self::State) {
        todo!()
    }

    fn pretty_print_inst(&self, state: &mut Self::State) -> std::prelude::v1::String {
        todo!()
    }
}
