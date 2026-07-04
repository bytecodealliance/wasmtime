use cranelift_control::ControlPlane;

use super::Inst;
use crate::{
    FrameLayout, MachBuffer, MachInstEmit, MachInstEmitState, ir,
    isa::arm32::abi::Arm32MachineDeps,
    machinst::{Callee, MachInst},
    settings,
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
        self._user_stack_map = user_stack_map;
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

/// Emit a single TrapCode (unconditional trap).
fn emit_trap(sink: &mut MachBuffer<Inst>) {
    sink.put2(0xBE00);
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Self>, _info: &Self::Info, _state: &mut Self::State) {
        match self {
            Inst::Ret => {
                // RET = BX LR: 0b0100_0111_0111_0000 = 0x4770
                sink.put2(0x4770);
            }

            // Rets is a pseudo-instruction that constrains return registers; it emits no bytes.
            Inst::Rets { .. } => {}

            // Args is a pseudo-instruction that defines arg registers; emits no bytes.
            Inst::Args { .. } => {}

            // Push registers — wide STMDB.W sp!, {list}: 0xE92D | reg_list.
            Inst::Push { rs } => {
                sink.put2(0xE92Du16);
                sink.put2(*rs);
            }

            // Pop registers — wide LDMIA.W sp!, {list}: 0xE8BD | reg_list.
            Inst::Pop { rt } => {
                sink.put2(0xE8BDu16);
                sink.put2(*rt);
            }
        }
        if self.is_trap() {
            emit_trap(sink);
        }
    }

    fn pretty_print_inst(&self, _state: &mut Self::State) -> std::prelude::v1::String {
        format!("{self:?}")
    }
}
