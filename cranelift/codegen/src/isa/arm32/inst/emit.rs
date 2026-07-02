//! arm32 (A32) ISA: binary code emission.

use crate::ir;
use crate::isa::arm32::inst::*;
use crate::settings;
use cranelift_control::ControlPlane;

/// Information carried along during emission, derived from the shared and ISA
/// flags.
pub struct EmitInfo {
    #[expect(dead_code, reason = "may be used once more instructions are added")]
    shared_flags: settings::Flags,
    #[expect(dead_code, reason = "may be used once more instructions are added")]
    isa_flags: super::super::settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(
        shared_flags: settings::Flags,
        isa_flags: super::super::settings::Flags,
    ) -> Self {
        Self {
            shared_flags,
            isa_flags,
        }
    }
}

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// The user stack map for the upcoming instruction.
    user_stack_map: Option<ir::UserStackMap>,
    /// Only used during fuzz-testing.
    ctrl_plane: ControlPlane,
    frame_layout: FrameLayout,
}

impl EmitState {
    #[expect(dead_code, reason = "will be used once safepoints are supported")]
    fn take_stack_map(&mut self) -> Option<ir::UserStackMap> {
        self.user_stack_map.take()
    }
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(
        abi: &Callee<crate::isa::arm32::abi::Arm32MachineDeps>,
        ctrl_plane: ControlPlane,
    ) -> Self {
        EmitState {
            user_stack_map: None,
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

/// Encoding of the "always" (AL) condition code, in the top nibble of an A32
/// instruction word.
const COND_AL: u32 = 0xe000_0000;

/// The hardware encoding number (0-15) of a real register.
fn machreg_to_gpr(reg: Reg) -> u32 {
    u32::from(reg.to_real_reg().unwrap().hw_enc() & 0xf)
}

/// `movw rd, #imm16` — load a 16-bit immediate into the low half of `rd`,
/// zeroing the high half.
fn enc_movw(rd: u32, imm16: u32) -> u32 {
    let imm4 = (imm16 >> 12) & 0xf;
    let imm12 = imm16 & 0xfff;
    COND_AL | 0x0300_0000 | (imm4 << 16) | (rd << 12) | imm12
}

/// `movt rd, #imm16` — load a 16-bit immediate into the high half of `rd`,
/// preserving the low half.
fn enc_movt(rd: u32, imm16: u32) -> u32 {
    let imm4 = (imm16 >> 12) & 0xf;
    let imm12 = imm16 & 0xfff;
    COND_AL | 0x0340_0000 | (imm4 << 16) | (rd << 12) | imm12
}

/// `mov rd, rm` (register).
fn enc_mov(rd: u32, rm: u32) -> u32 {
    COND_AL | 0x01a0_0000 | (rd << 12) | rm
}

/// `bx lr` — branch and exchange to the link register.
fn enc_bx_lr() -> u32 {
    COND_AL | 0x012f_ff10 | 14
}

/// `add`/`sub sp, sp, #imm` for a small `imm` (< 256, encoded with rotation 0).
fn enc_sp_adjust(amount: i32) -> u32 {
    let sp = 13u32;
    let (base, mag) = if amount < 0 {
        (0x0240_0000u32, (-amount) as u32) // sub
    } else {
        (0x0280_0000u32, amount as u32) // add
    };
    assert!(
        mag < 256,
        "arm32 sp adjust out of simple-immediate range: {amount}"
    );
    COND_AL | base | (sp << 16) | (sp << 12) | mag
}

/// `str`/`ldr rt, [base, #offset]` with a 12-bit immediate offset.
fn enc_ldr_str(load: bool, rt: u32, base: u32, offset: i32) -> u32 {
    let (u_bit, mag) = if offset < 0 {
        (0u32, (-offset) as u32)
    } else {
        (1u32, offset as u32)
    };
    assert!(mag < 4096, "arm32 ldr/str offset out of range: {offset}");
    // cond 01 I(0) P(1) U W(0) B(0) L rn rt imm12
    let l_bit = if load { 1u32 } else { 0 };
    COND_AL
        | 0x0400_0000
        | (1 << 24) // P = 1 (pre-indexed)
        | (u_bit << 23)
        | (l_bit << 20)
        | (base << 16)
        | (rt << 12)
        | mag
}

/// `b label` — unconditional branch (offset patched in later).
fn enc_b_placeholder() -> u32 {
    COND_AL | 0x0a00_0000
}

fn put_u32(sink: &mut MachBuffer<Inst>, word: u32) {
    for b in word.to_le_bytes() {
        sink.put1(b);
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Inst>, _emit_info: &Self::Info, _state: &mut EmitState) {
        let start = sink.cur_offset();
        match self {
            Inst::Nop0 => {}
            Inst::Nop4 => put_u32(sink, COND_AL | 0x0320_f000),
            Inst::Ret => put_u32(sink, enc_bx_lr()),
            Inst::MovImm { rd, imm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                put_u32(sink, enc_movw(rd, (imm & 0xffff) as u32));
                if imm >> 16 != 0 {
                    put_u32(sink, enc_movt(rd, (imm >> 16) as u32));
                }
            }
            Inst::Mov { rd, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_mov(rd, rm));
            }
            Inst::AdjustSp { amount } => put_u32(sink, enc_sp_adjust(*amount)),
            Inst::Store { rt, base, offset } => {
                let rt = machreg_to_gpr(*rt);
                let base = machreg_to_gpr(*base);
                put_u32(sink, enc_ldr_str(false, rt, base, *offset));
            }
            Inst::Load { rt, base, offset } => {
                let rt = machreg_to_gpr(rt.to_reg());
                let base = machreg_to_gpr(*base);
                put_u32(sink, enc_ldr_str(true, rt, base, *offset));
            }
            Inst::Jump { dest } => {
                sink.use_label_at_offset(sink.cur_offset(), *dest, LabelUse::Branch26);
                sink.add_uncond_branch(sink.cur_offset(), sink.cur_offset() + 4, *dest);
                put_u32(sink, enc_b_placeholder());
            }
            Inst::Args { .. } | Inst::Rets { .. } => {
                // Pseudo-instructions: no machine code.
            }
        }

        let end = sink.cur_offset();
        debug_assert!(
            (end - start) <= Inst::worst_case_size(),
            "instruction {self:?} exceeded worst-case size"
        );
    }

    fn pretty_print_inst(&self, state: &mut Self::State) -> String {
        self.print_with_state(state)
    }
}
