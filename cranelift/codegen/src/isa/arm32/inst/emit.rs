//! arm32 (A32) ISA: binary code emission.

use crate::binemit::Reloc;
use crate::ir;
use crate::ir::AtomicRmwOp;
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

//=============================================================================
// Instruction encoders.

/// A data-processing instruction with an immediate operand2 (`op{s} rd, rn,
/// #imm12`). `imm12` is the already-encoded rotated immediate.
fn enc_dp_imm(opcode: u32, s: u32, rd: u32, rn: u32, imm12: u32) -> u32 {
    COND_AL | (1 << 25) | (opcode << 21) | (s << 20) | (rn << 16) | (rd << 12) | (imm12 & 0xfff)
}

/// A data-processing instruction with a register operand2 (`op{s} rd, rn, rm`).
fn enc_dp_reg(opcode: u32, s: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    COND_AL | (opcode << 21) | (s << 20) | (rn << 16) | (rd << 12) | rm
}

/// `movw rd, #imm16` — load a 16-bit immediate into the low half of `rd`.
fn enc_movw(rd: u32, imm16: u32) -> u32 {
    let imm4 = (imm16 >> 12) & 0xf;
    let imm12 = imm16 & 0xfff;
    COND_AL | 0x0300_0000 | (imm4 << 16) | (rd << 12) | imm12
}

/// `movt rd, #imm16` — load a 16-bit immediate into the high half of `rd`.
fn enc_movt(rd: u32, imm16: u32) -> u32 {
    let imm4 = (imm16 >> 12) & 0xf;
    let imm12 = imm16 & 0xfff;
    COND_AL | 0x0340_0000 | (imm4 << 16) | (rd << 12) | imm12
}

/// `bx rm` — branch and exchange.
fn enc_bx(rm: u32) -> u32 {
    COND_AL | 0x012f_ff10 | rm
}

/// `b <imm24>` — unconditional branch (offset patched in later).
fn enc_b(imm24: u32) -> u32 {
    COND_AL | 0x0a00_0000 | (imm24 & 0x00ff_ffff)
}

/// `b<cond> <imm24>` — conditional branch.
fn enc_bcond(cond: Cond, imm24: u32) -> u32 {
    (cond.bits() << 28) | 0x0a00_0000 | (imm24 & 0x00ff_ffff)
}

/// `bl <imm24>` — branch with link (direct call).
fn enc_bl(imm24: u32) -> u32 {
    COND_AL | 0x0b00_0000 | (imm24 & 0x00ff_ffff)
}

/// `blx rm` — branch with link and exchange to a register (indirect call).
fn enc_blx(rm: u32) -> u32 {
    COND_AL | 0x012f_ff30 | rm
}

/// Common post-call sequence: record the safepoint/call site, pop callee stack
/// args, and load any stack-carried return values.
fn emit_call_epilogue<T>(
    sink: &mut MachBuffer<Inst>,
    emit_info: &EmitInfo,
    state: &mut EmitState,
    info: &crate::machinst::CallInfo<T>,
) {
    use crate::isa::arm32::abi::Arm32MachineDeps;
    use crate::machinst::ABIMachineSpec;

    if let Some(s) = state.take_stack_map() {
        let offset = sink.cur_offset();
        sink.push_user_stack_map(state, offset, s);
    }
    sink.add_call_site();

    let callee_pop_size = i32::try_from(info.callee_pop_size).unwrap();
    if callee_pop_size > 0 {
        for inst in Arm32MachineDeps::gen_sp_reg_adjust(-callee_pop_size) {
            inst.emit(sink, emit_info, state);
        }
    }

    info.emit_retval_loads::<Arm32MachineDeps, _, _>(
        state.frame_layout().stackslots_size,
        |inst| inst.emit(sink, emit_info, state),
        |_needed_space| None,
    );
}

/// `add`/`sub sp, sp, #imm`.
fn enc_sp_adjust(amount: i32) -> u32 {
    let (op, mag) = if amount < 0 {
        (ALUOp::Sub, (-amount) as u32)
    } else {
        (ALUOp::Add, amount as u32)
    };
    let imm12 =
        encode_rotated_imm(mag).expect("arm32 sp adjust not encodable as a single immediate");
    enc_dp_imm(op.opcode(), 0, 13, 13, imm12)
}

/// `ldr`/`str{b} rt, [base, #±offset]` (word or byte).
fn enc_ldr_str_imm(load: bool, byte: bool, rt: u32, base: u32, offset: i32) -> u32 {
    let (u_bit, mag) = if offset < 0 {
        (0u32, (-offset) as u32)
    } else {
        (1u32, offset as u32)
    };
    assert!(mag < 4096, "arm32 ldr/str offset out of range: {offset}");
    COND_AL
        | 0x0400_0000
        | (1 << 24) // P = 1 (pre-indexed)
        | (u_bit << 23)
        | (if byte { 1 } else { 0 } << 22)
        | (if load { 1 } else { 0 } << 20)
        | (base << 16)
        | (rt << 12)
        | mag
}

/// `ldr`/`str{b} rt, [base, index]` (word or byte, register offset).
fn enc_ldr_str_reg(load: bool, byte: bool, rt: u32, base: u32, index: u32) -> u32 {
    COND_AL
        | 0x0600_0000
        | (1 << 24) // P = 1
        | (1 << 23) // U = 1 (add)
        | (if byte { 1 } else { 0 } << 22)
        | (if load { 1 } else { 0 } << 20)
        | (base << 16)
        | (rt << 12)
        | index
}

/// Extra load/store (halfword and signed byte) with an 8-bit immediate offset.
fn enc_ldrh_strh_imm(load: bool, s: u32, h: u32, rt: u32, base: u32, offset: i32) -> u32 {
    let (u_bit, mag) = if offset < 0 {
        (0u32, (-offset) as u32)
    } else {
        (1u32, offset as u32)
    };
    assert!(mag < 256, "arm32 ldrh/strh offset out of range: {offset}");
    let imm_h = (mag >> 4) & 0xf;
    let imm_l = mag & 0xf;
    COND_AL
        | (1 << 24) // P = 1
        | (u_bit << 23)
        | (1 << 22) // immediate form
        | (if load { 1 } else { 0 } << 20)
        | (base << 16)
        | (rt << 12)
        | (imm_h << 8)
        | (1 << 7)
        | (s << 6)
        | (h << 5)
        | (1 << 4)
        | imm_l
}

/// `push {reglist}` (STMDB sp!).
fn enc_push(reg_list: u32) -> u32 {
    COND_AL | 0x092d_0000 | (reg_list & 0xffff)
}

/// `pop {reglist}` (LDMIA sp!).
fn enc_pop(reg_list: u32) -> u32 {
    COND_AL | 0x08bd_0000 | (reg_list & 0xffff)
}

/// A shift/rotate by a constant amount: `mov rd, rm, <shift> #amount`.
fn enc_shift_imm(op: ShiftOp, rd: u32, rm: u32, amount: u32) -> u32 {
    COND_AL | 0x01a0_0000 | (rd << 12) | ((amount & 0x1f) << 7) | (op.bits() << 5) | rm
}

/// A shift/rotate by a register amount: `mov rd, rm, <shift> rs`.
fn enc_shift_reg(op: ShiftOp, rd: u32, rm: u32, rs: u32) -> u32 {
    COND_AL | 0x01a0_0000 | (rd << 12) | (rs << 8) | (op.bits() << 5) | (1 << 4) | rm
}

/// `mul rd, rn, rm` — `rd = rn * rm`.
fn enc_mul(rd: u32, rn: u32, rm: u32) -> u32 {
    COND_AL | (rd << 16) | (rm << 8) | 0x90 | rn
}

/// `mla rd, rn, rm, ra` — `rd = rn * rm + ra`.
fn enc_mla(rd: u32, rn: u32, rm: u32, ra: u32) -> u32 {
    COND_AL | 0x0020_0000 | (rd << 16) | (ra << 12) | (rm << 8) | 0x90 | rn
}

/// A 32x32->64 long multiply (`umull`/`smull`). `signed` selects `smull`.
fn enc_mull(signed: bool, rd_lo: u32, rd_hi: u32, rn: u32, rm: u32) -> u32 {
    let base = if signed { 0x00c0_0000 } else { 0x0080_0000 };
    COND_AL | base | (rd_hi << 16) | (rd_lo << 12) | (rm << 8) | 0x90 | rn
}

/// A 32x32->64 multiply-accumulate (`umlal`/`smlal`). `signed` selects `smlal`.
fn enc_mlal(signed: bool, rd_lo: u32, rd_hi: u32, rn: u32, rm: u32) -> u32 {
    let base = if signed { 0x00e0_0000 } else { 0x00a0_0000 };
    COND_AL | base | (rd_hi << 16) | (rd_lo << 12) | (rm << 8) | 0x90 | rn
}

/// `mls rd, rn, rm, ra` — `rd = ra - rn * rm`.
fn enc_mls(rd: u32, rn: u32, rm: u32, ra: u32) -> u32 {
    COND_AL | 0x0060_0090 | (rd << 16) | (ra << 12) | (rm << 8) | rn
}

/// `sdiv`/`udiv rd, rn, rm`. `signed` selects `sdiv`.
fn enc_div(signed: bool, rd: u32, rn: u32, rm: u32) -> u32 {
    let base = if signed { 0x0710_f010 } else { 0x0730_f010 };
    COND_AL | base | (rd << 16) | (rm << 8) | rn
}

/// A single-operand `op rd, rm` built from an opcode template.
fn enc_op_rd_rm(template: u32, rd: u32, rm: u32) -> u32 {
    COND_AL | template | (rd << 12) | rm
}

/// A conditional register move: `mov<cond> rd, rm`.
fn enc_mov_cond(cond: Cond, rd: u32, rm: u32) -> u32 {
    (cond.bits() << 28) | 0x01a0_0000 | (rd << 12) | rm
}

/// `bfc rd, #lsb, #width`.
fn enc_bfc(rd: u32, lsb: u32, width: u32) -> u32 {
    let msb = lsb + width - 1;
    COND_AL | 0x07c0_001f | (msb << 16) | (rd << 12) | (lsb << 7)
}

/// `bfi rd, rn, #lsb, #width`.
fn enc_bfi(rd: u32, rn: u32, lsb: u32, width: u32) -> u32 {
    let msb = lsb + width - 1;
    COND_AL | 0x07c0_0010 | (msb << 16) | (rd << 12) | (lsb << 7) | rn
}

/// `sbfx`/`ubfx rd, rn, #lsb, #width` from the op template.
fn enc_bfx(template: u32, rd: u32, rn: u32, lsb: u32, width: u32) -> u32 {
    COND_AL | template | ((width - 1) << 16) | (rd << 12) | (lsb << 7) | rn
}

/// A three-register op `template rd, rn, rm` with `rn`/`rd`/`rm` zeroed
/// (used by the saturating, parallel, sel, pkh and extend-add families).
fn enc_rd_rn_rm(template: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    COND_AL | template | (rn << 16) | (rd << 12) | rm
}

/// `ssat`/`usat rd, #sat_bits, rm`.
fn enc_sat(op: SatOp, rd: u32, sat_bits: u32, rm: u32) -> u32 {
    let field = if op.is_signed() {
        sat_bits - 1
    } else {
        sat_bits
    };
    COND_AL | op.template() | (field << 16) | (rd << 12) | rm
}

/// `rrx rd, rm` (a `mov` with a rotate-right-extend shift).
fn enc_rrx(rd: u32, rm: u32) -> u32 {
    COND_AL | 0x01a0_0060 | (rd << 12) | rm
}

/// The permanently-undefined `udf` encoding (`udf #0`).
fn enc_udf() -> u32 {
    0xe7f0_00f0
}

/// A DSP multiply with a single result: `template` plus `rd`(19:16),
/// `rm`(11:8), `rn`(3:0).
fn enc_dsp3(template: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    COND_AL | template | (rd << 16) | (rm << 8) | rn
}

/// A DSP multiply-accumulate: `template` plus `rd`(19:16), `ra`(15:12),
/// `rm`(11:8), `rn`(3:0).
fn enc_dsp4(template: u32, rd: u32, rn: u32, rm: u32, ra: u32) -> u32 {
    COND_AL | template | (rd << 16) | (ra << 12) | (rm << 8) | rn
}

/// A DSP long multiply: `template` plus `rd_hi`(19:16), `rd_lo`(15:12),
/// `rm`(11:8), `rn`(3:0).
fn enc_dsp_long(template: u32, rd_lo: u32, rd_hi: u32, rn: u32, rm: u32) -> u32 {
    COND_AL | template | (rd_hi << 16) | (rd_lo << 12) | (rm << 8) | rn
}

/// `ldmia`/`stmia rn{!}, {reglist}` (increment-after).
fn enc_ldm_stm(load: bool, rn: u32, writeback: bool, reg_list: u32) -> u32 {
    let base = if load { 0x0890_0000 } else { 0x0880_0000 };
    let wb = if writeback { 0x0020_0000 } else { 0 };
    COND_AL | base | wb | (rn << 16) | (reg_list & 0xffff)
}

/// `ldrex{b,h,d}`/`ldaex{b,h,d} rt, [rn]` (the size field selects the width).
fn enc_load_ex(acquire: bool, size: AtomicSize, rt: u32, rn: u32) -> u32 {
    let base = if acquire { 0x0190_0e9f } else { 0x0190_0f9f };
    COND_AL | base | size.enc_bits() | (rn << 16) | (rt << 12)
}

/// `strex{b,h,d}`/`stlex{b,h,d} rd, rt, [rn]`.
fn enc_store_ex(release: bool, size: AtomicSize, rd: u32, rt: u32, rn: u32) -> u32 {
    let base = if release { 0x0180_0e90 } else { 0x0180_0f90 };
    COND_AL | base | size.enc_bits() | (rn << 16) | (rd << 12) | rt
}

/// `lda{b,h} rt, [rn]`.
fn enc_lda(size: AtomicSize, rt: u32, rn: u32) -> u32 {
    COND_AL | 0x0190_0c9f | size.enc_bits() | (rn << 16) | (rt << 12)
}

/// `stl{b,h} rt, [rn]`.
fn enc_stl(size: AtomicSize, rt: u32, rn: u32) -> u32 {
    COND_AL | 0x0180_fc90 | size.enc_bits() | (rn << 16) | rt
}

/// The hardware encoding number (0-15) of a VFP register. An f32 uses the low
/// S-half of its D register, whose Vd/D fields encode identically, so this is
/// just the D-register number.
fn machreg_to_vfp(reg: Reg) -> u32 {
    u32::from(reg.to_real_reg().unwrap().hw_enc() & 0xf)
}

/// A three-register VFP data-processing instruction.
fn enc_fpu_rrr(op: FpuOp3, size: FpuSize, d: u32, n: u32, m: u32) -> u32 {
    op.base() | size.size_bit() | (n << 16) | (d << 12) | m
}

/// A two-register VFP data-processing instruction.
fn enc_fpu_rr(op: FpuOp2, size: FpuSize, d: u32, m: u32) -> u32 {
    op.base() | size.size_bit() | (d << 12) | m
}

/// `vldr`/`vstr <reg>, [rn, #±offset]` (offset must be a multiple of 4).
fn enc_vldr_vstr(load: bool, size: FpuSize, vd: u32, rn: u32, offset: i32) -> u32 {
    let base = (if load { 0xed90_0a00 } else { 0xed80_0a00 }) | size.size_bit();
    assert!(offset % 4 == 0, "arm32 vldr/vstr offset must be 4-aligned");
    let (base, mag) = if offset < 0 {
        (base & !0x0080_0000, ((-offset) as u32) / 4)
    } else {
        (base, (offset as u32) / 4)
    };
    assert!(mag < 256, "arm32 vldr/vstr offset out of range");
    base | (rn << 16) | (vd << 12) | mag
}

/// `vmov sn, rt` — move a GPR into a single-precision register.
fn enc_vmov_gpr_s(vn: u32, rt: u32) -> u32 {
    COND_AL | 0x0e00_0a10 | (vn << 16) | (rt << 12)
}

/// `vmov dm, rt_lo, rt_hi` — move two GPRs into a double register.
fn enc_vmov_gpr_d(vm: u32, rt_lo: u32, rt_hi: u32) -> u32 {
    COND_AL | 0x0c40_0b10 | (rt_hi << 16) | (rt_lo << 12) | vm
}

/// `vmov rt, sn` — move a single register into a GPR.
fn enc_vmov_s_to_gpr(rt: u32, vn: u32) -> u32 {
    COND_AL | 0x0e10_0a10 | (vn << 16) | (rt << 12)
}

/// `vmov rt_lo, rt_hi, dm` — move a double register into two GPRs.
fn enc_vmov_d_to_gpr(rt_lo: u32, rt_hi: u32, vm: u32) -> u32 {
    COND_AL | 0x0c50_0b10 | (rt_hi << 16) | (rt_lo << 12) | vm
}

/// The `vmrs APSR_nzcv, FPSCR` instruction (moves the VFP flags to the CPSR).
const VMRS_APSR_NZCV: u32 = 0xeef1_fa10;

/// `vcmp.f32/f64 sn/dn, sm/dm`.
fn enc_vcmp(size: FpuSize, n: u32, m: u32) -> u32 {
    (0xeeb4_0a40 | size.size_bit()) | (n << 12) | m
}

/// `vcvt.f64.f32`/`vcvt.f32.f64` (`to_f64` selects promote vs demote).
fn enc_vcvt_ff(to_f64: bool, d: u32, m: u32) -> u32 {
    let base = if to_f64 { 0xeeb7_0ac0 } else { 0xeeb7_0bc0 };
    base | (d << 12) | m
}

/// `vcvt.s32/u32.f32/f64` — float to integer, round toward zero.
fn enc_vcvt_to_int(signed: bool, size: FpuSize, d: u32, m: u32) -> u32 {
    let sign = if signed { 0x1_0000 } else { 0 };
    (0xeebc_0ac0 | sign | size.size_bit()) | (d << 12) | m
}

/// `vcvt.f32/f64.s32/u32` — integer (in the S reg `m`) to float.
fn enc_vcvt_from_int(signed: bool, size: FpuSize, d: u32, m: u32) -> u32 {
    let sign = if signed { 0x80 } else { 0 };
    (0xeeb8_0a40 | sign | size.size_bit()) | (d << 12) | m
}

/// `vmov<cond>.f64 dd, dm` — a conditional double-register move.
fn enc_vmov_f64_cond(cond: Cond, d: u32, m: u32) -> u32 {
    (cond.bits() << 28) | 0x0eb0_0b40 | (d << 12) | m
}

/// `vminnm`/`vmaxnm` (IEEE number min/max; ARMv8). These are unconditional.
fn enc_fpu_minmax(max: bool, size: FpuSize, d: u32, n: u32, m: u32) -> u32 {
    let op = if max { 0 } else { 0x40 };
    (0xfe80_0a00 | op | size.size_bit()) | (n << 16) | (d << 12) | m
}

fn put_u32(sink: &mut MachBuffer<Inst>, word: u32) {
    for b in word.to_le_bytes() {
        sink.put1(b);
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Inst>, emit_info: &Self::Info, state: &mut EmitState) {
        let start = sink.cur_offset();
        match self {
            Inst::Nop0 => {}
            Inst::Nop4 => put_u32(sink, COND_AL | 0x0320_f000),
            Inst::Ret => put_u32(sink, enc_bx(14)),

            Inst::MovImm { rd, imm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                put_u32(sink, enc_movw(rd, (imm & 0xffff) as u32));
                if imm >> 16 != 0 {
                    put_u32(sink, enc_movt(rd, (imm >> 16) as u32));
                }
            }
            Inst::MovRotImm { rd, imm12 } => {
                let rd = machreg_to_gpr(rd.to_reg());
                put_u32(sink, enc_dp_imm(0b1101, 0, rd, 0, *imm12));
            }
            Inst::MvnRotImm { rd, imm12 } => {
                let rd = machreg_to_gpr(rd.to_reg());
                put_u32(sink, enc_dp_imm(0b1111, 0, rd, 0, *imm12));
            }
            Inst::Movw { rd, imm16 } => {
                put_u32(sink, enc_movw(machreg_to_gpr(rd.to_reg()), *imm16));
            }
            Inst::Movt { rd, imm16 } => {
                put_u32(sink, enc_movt(machreg_to_gpr(rd.to_reg()), *imm16));
            }
            Inst::MovReg { rd, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dp_reg(0b1101, 0, rd, 0, rm));
            }
            Inst::MvnReg { rd, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dp_reg(0b1111, 0, rd, 0, rm));
            }

            Inst::AluRRR { op, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dp_reg(op.opcode(), 0, rd, rn, rm));
            }
            Inst::AluRRImm { op, rd, rn, imm12 } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_dp_imm(op.opcode(), 0, rd, rn, *imm12));
            }
            Inst::AluRRRFlags { op, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dp_reg(op.opcode(), 1, rd, rn, rm));
            }
            Inst::ShiftImm { op, rd, rm, amount } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_shift_imm(*op, rd, rm, u32::from(*amount)));
            }
            Inst::ShiftReg { op, rd, rm, rs } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                let rs = machreg_to_gpr(*rs);
                put_u32(sink, enc_shift_reg(*op, rd, rm, rs));
            }
            Inst::Mul { rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_mul(rd, rn, rm));
            }
            Inst::Mla { rd, rn, rm, ra } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                let ra = machreg_to_gpr(*ra);
                put_u32(sink, enc_mla(rd, rn, rm, ra));
            }
            Inst::Umull {
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => {
                let rd_lo = machreg_to_gpr(rd_lo.to_reg());
                let rd_hi = machreg_to_gpr(rd_hi.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_mull(false, rd_lo, rd_hi, rn, rm));
            }
            Inst::Smull {
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => {
                let rd_lo = machreg_to_gpr(rd_lo.to_reg());
                let rd_hi = machreg_to_gpr(rd_hi.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_mull(true, rd_lo, rd_hi, rn, rm));
            }
            Inst::Umlal {
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => {
                let rd_lo = machreg_to_gpr(rd_lo.to_reg());
                let rd_hi = machreg_to_gpr(rd_hi.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_mlal(false, rd_lo, rd_hi, rn, rm));
            }
            Inst::Smlal {
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => {
                let rd_lo = machreg_to_gpr(rd_lo.to_reg());
                let rd_hi = machreg_to_gpr(rd_hi.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_mlal(true, rd_lo, rd_hi, rn, rm));
            }
            Inst::Mls { rd, rn, rm, ra } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                let ra = machreg_to_gpr(*ra);
                put_u32(sink, enc_mls(rd, rn, rm, ra));
            }
            Inst::SDiv { rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_div(true, rd, rn, rm));
            }
            Inst::UDiv { rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_div(false, rd, rn, rm));
            }
            Inst::BitRR { op, rd, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_op_rd_rm(op.template(), rd, rm));
            }
            Inst::ExtRR { op, rd, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_op_rd_rm(op.template(), rd, rm));
            }
            Inst::CSel { cond, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                // Default to the "false" value, then conditionally overwrite.
                put_u32(sink, enc_dp_reg(0b1101, 0, rd, 0, rm));
                put_u32(sink, enc_mov_cond(*cond, rd, rn));
            }
            Inst::Bfc { rd, lsb, width } => {
                let rd = machreg_to_gpr(rd.to_reg());
                put_u32(sink, enc_bfc(rd, u32::from(*lsb), u32::from(*width)));
            }
            Inst::Bfi { rd, rn, lsb, width } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_bfi(rd, rn, u32::from(*lsb), u32::from(*width)));
            }
            Inst::Bfx {
                op,
                rd,
                rn,
                lsb,
                width,
            } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                put_u32(
                    sink,
                    enc_bfx(op.template(), rd, rn, u32::from(*lsb), u32::from(*width)),
                );
            }
            Inst::QAlu { op, rd, rm, rn } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_rd_rn_rm(op.template(), rd, rn, rm));
            }
            Inst::Sat {
                op,
                rd,
                sat_bits,
                rm,
            } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_sat(*op, rd, u32::from(*sat_bits), rm));
            }
            Inst::ParAlu { op, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_rd_rn_rm(op.template(), rd, rn, rm));
            }
            Inst::ExtAdd { op, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_rd_rn_rm(op.template(), rd, rn, rm));
            }
            Inst::DspMul3 { op, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dsp3(op.template(), rd, rn, rm));
            }
            Inst::DspMul4 { op, rd, rn, rm, ra } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                let ra = machreg_to_gpr(*ra);
                put_u32(sink, enc_dsp4(op.template(), rd, rn, rm, ra));
            }
            Inst::DspMulL {
                op,
                rd_lo,
                rd_hi,
                rn,
                rm,
            } => {
                let rd_lo = machreg_to_gpr(rd_lo.to_reg());
                let rd_hi = machreg_to_gpr(rd_hi.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dsp_long(op.template(), rd_lo, rd_hi, rn, rm));
            }
            Inst::Sel { rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_rd_rn_rm(0x0680_0fb0, rd, rn, rm));
            }
            Inst::Pkh { op, rd, rn, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_rd_rn_rm(op.template(), rd, rn, rm));
            }
            Inst::Rrx { rd, rm } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_rrx(rd, rm));
            }
            Inst::Udf { code } => {
                sink.add_trap(*code);
                put_u32(sink, enc_udf());
            }
            Inst::LdmStm {
                load,
                rn,
                writeback,
                reg_list,
            } => {
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_ldm_stm(*load, rn, *writeback, *reg_list));
            }
            Inst::LoadEx {
                acquire,
                size,
                rt,
                rn,
            } => {
                let rt = machreg_to_gpr(rt.to_reg());
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_load_ex(*acquire, *size, rt, rn));
            }
            Inst::StoreEx {
                release,
                size,
                rd,
                rt,
                rn,
            } => {
                let rd = machreg_to_gpr(rd.to_reg());
                let rt = machreg_to_gpr(*rt);
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_store_ex(*release, *size, rd, rt, rn));
            }
            Inst::LoadAcq { size, rt, rn } => {
                let rt = machreg_to_gpr(rt.to_reg());
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_lda(*size, rt, rn));
            }
            Inst::StoreRel { size, rt, rn } => {
                let rt = machreg_to_gpr(*rt);
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_stl(*size, rt, rn));
            }
            Inst::Barrier { op } => put_u32(sink, op.encoding()),
            Inst::AtomicRmw {
                op,
                size,
                rd,
                addr,
                operand,
                tmp1,
                tmp2,
            } => emit_atomic_rmw(
                sink,
                state,
                *op,
                *size,
                machreg_to_gpr(rd.to_reg()),
                machreg_to_gpr(*addr),
                machreg_to_gpr(*operand),
                machreg_to_gpr(tmp1.to_reg()),
                machreg_to_gpr(tmp2.to_reg()),
            ),
            Inst::AtomicCas {
                size,
                rd,
                addr,
                expected,
                new,
                tmp,
            } => emit_atomic_cas(
                sink,
                state,
                *size,
                machreg_to_gpr(rd.to_reg()),
                machreg_to_gpr(*addr),
                machreg_to_gpr(*expected),
                machreg_to_gpr(*new),
                machreg_to_gpr(tmp.to_reg()),
            ),
            Inst::AtomicRmw64 {
                op,
                rd_lo,
                rd_hi,
                addr,
                operand_lo,
                operand_hi,
                tmp_new_lo,
                tmp_new_hi,
                tmp_res,
            } => emit_atomic_rmw64(
                sink,
                state,
                *op,
                machreg_to_gpr(rd_lo.to_reg()),
                machreg_to_gpr(rd_hi.to_reg()),
                machreg_to_gpr(*addr),
                machreg_to_gpr(*operand_lo),
                machreg_to_gpr(*operand_hi),
                machreg_to_gpr(tmp_new_lo.to_reg()),
                machreg_to_gpr(tmp_new_hi.to_reg()),
                machreg_to_gpr(tmp_res.to_reg()),
            ),
            Inst::AtomicCas64 {
                rd_lo,
                rd_hi,
                addr,
                expected_lo,
                expected_hi,
                new_lo,
                new_hi,
                tmp_res,
            } => emit_atomic_cas64(
                sink,
                state,
                machreg_to_gpr(rd_lo.to_reg()),
                machreg_to_gpr(rd_hi.to_reg()),
                machreg_to_gpr(*addr),
                machreg_to_gpr(*expected_lo),
                machreg_to_gpr(*expected_hi),
                machreg_to_gpr(*new_lo),
                machreg_to_gpr(*new_hi),
                machreg_to_gpr(tmp_res.to_reg()),
            ),
            Inst::AtomicLoad64 { rd_lo, rd_hi, addr } => {
                let rd_lo = machreg_to_gpr(rd_lo.to_reg());
                debug_assert_eq!(machreg_to_gpr(rd_hi.to_reg()), rd_lo + 1);
                let addr = machreg_to_gpr(*addr);
                // ldaexd rd_lo, rd_hi, [addr]; clrex
                put_u32(sink, enc_load_ex(true, AtomicSize::DWord, rd_lo, addr));
                put_u32(sink, BarrierOp::Clrex.encoding());
            }
            Inst::AtomicStore64 {
                addr,
                src_lo,
                src_hi,
                scratch_lo,
                scratch_hi,
                tmp_res,
            } => emit_atomic_store64(
                sink,
                state,
                machreg_to_gpr(*addr),
                machreg_to_gpr(*src_lo),
                machreg_to_gpr(*src_hi),
                machreg_to_gpr(scratch_lo.to_reg()),
                machreg_to_gpr(scratch_hi.to_reg()),
                machreg_to_gpr(tmp_res.to_reg()),
            ),

            Inst::FpuRRR {
                op,
                size,
                rd,
                rn,
                rm,
            } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rn = machreg_to_vfp(*rn);
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_fpu_rrr(*op, *size, rd, rn, rm));
            }
            Inst::FpuRR { op, size, rd, rm } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_fpu_rr(*op, *size, rd, rm));
            }
            Inst::FpuLoad { size, rd, mem } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let (base, offset) = resolve_fpu_amode(mem, state);
                put_u32(sink, enc_vldr_vstr(true, *size, rd, base, offset));
            }
            Inst::FpuStore { size, rt, mem } => {
                let rt = machreg_to_vfp(*rt);
                let (base, offset) = resolve_fpu_amode(mem, state);
                put_u32(sink, enc_vldr_vstr(false, *size, rt, base, offset));
            }
            Inst::MovToFpu32 { rd, rt } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rt = machreg_to_gpr(*rt);
                put_u32(sink, enc_vmov_gpr_s(rd, rt));
            }
            Inst::MovToFpu64 { rd, rt_lo, rt_hi } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rt_lo = machreg_to_gpr(*rt_lo);
                let rt_hi = machreg_to_gpr(*rt_hi);
                put_u32(sink, enc_vmov_gpr_d(rd, rt_lo, rt_hi));
            }
            Inst::MovFromFpu32 { rt, rm } => {
                let rt = machreg_to_gpr(rt.to_reg());
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_vmov_s_to_gpr(rt, rm));
            }
            Inst::MovFromFpu64 { rt_lo, rt_hi, rm } => {
                let rt_lo = machreg_to_gpr(rt_lo.to_reg());
                let rt_hi = machreg_to_gpr(rt_hi.to_reg());
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_vmov_d_to_gpr(rt_lo, rt_hi, rm));
            }
            Inst::VcmpMrs { size, rn, rm } => {
                let rn = machreg_to_vfp(*rn);
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_vcmp(*size, rn, rm));
                put_u32(sink, VMRS_APSR_NZCV);
            }
            Inst::VcvtFF { to_f64, rd, rm } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_vcvt_ff(*to_f64, rd, rm));
            }
            Inst::VcvtToInt {
                signed,
                size,
                rd,
                rm,
            } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_vcvt_to_int(*signed, *size, rd, rm));
            }
            Inst::VcvtFromInt {
                signed,
                size,
                rd,
                rm,
            } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_vcvt_from_int(*signed, *size, rd, rm));
            }
            Inst::FpuCSel { cond, rd, rn, rm } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rn = machreg_to_vfp(*rn);
                let rm = machreg_to_vfp(*rm);
                // Default to `if_false`, then conditionally overwrite.
                put_u32(sink, enc_fpu_rr(FpuOp2::Vmov, FpuSize::F64, rd, rm));
                put_u32(sink, enc_vmov_f64_cond(*cond, rd, rn));
            }
            Inst::FpuMinMax {
                max,
                size,
                rd,
                rn,
                rm,
            } => {
                let rd = machreg_to_vfp(rd.to_reg());
                let rn = machreg_to_vfp(*rn);
                let rm = machreg_to_vfp(*rm);
                put_u32(sink, enc_fpu_minmax(*max, *size, rd, rn, rm));
            }
            Inst::CmpRR { op, rn, rm } => {
                let rn = machreg_to_gpr(*rn);
                let rm = machreg_to_gpr(*rm);
                put_u32(sink, enc_dp_reg(op.opcode(), 1, 0, rn, rm));
            }
            Inst::CmpRImm { op, rn, imm12 } => {
                let rn = machreg_to_gpr(*rn);
                put_u32(sink, enc_dp_imm(op.opcode(), 1, 0, rn, *imm12));
            }

            Inst::AdjustSp { amount } => put_u32(sink, enc_sp_adjust(*amount)),

            Inst::Load { rt, mem, kind } => {
                let rt = machreg_to_gpr(rt.to_reg());
                emit_load_store(sink, rt, mem, state, LoadStore::Load(*kind));
            }
            Inst::Store { rt, mem, kind } => {
                let rt = machreg_to_gpr(*rt);
                emit_load_store(sink, rt, mem, state, LoadStore::Store(*kind));
            }

            Inst::Push { reg_list } => put_u32(sink, enc_push(*reg_list)),
            Inst::Pop { reg_list } => put_u32(sink, enc_pop(*reg_list)),

            Inst::Call { info } => {
                sink.add_reloc(Reloc::Arm32Call, &info.dest, 0);
                put_u32(sink, enc_bl(0));
                emit_call_epilogue(sink, emit_info, state, info);
            }
            Inst::CallInd { info } => {
                let rm = machreg_to_gpr(info.dest);
                put_u32(sink, enc_blx(rm));
                emit_call_epilogue(sink, emit_info, state, info);
            }
            Inst::LoadExtName { rd, name, offset } => {
                let rd = machreg_to_gpr(rd.to_reg());
                // `ldr rd, [pc]` reads `pc` as this instruction's address + 8,
                // i.e. the inline literal placed two slots ahead.
                put_u32(sink, enc_ldr_str_imm(true, false, rd, 15, 0));
                // `b .+8` branches over the literal (its `pc` is already the
                // instruction just past the `.word`, so the offset is 0).
                put_u32(sink, enc_b(0));
                // The literal: the symbol's absolute address plus `offset`,
                // supplied by the linker via an `Abs4` (32-bit absolute) reloc.
                sink.add_reloc(Reloc::Abs4, name.as_ref(), *offset);
                put_u32(sink, 0);
            }

            Inst::Jump { dest } => {
                sink.use_label_at_offset(sink.cur_offset(), *dest, LabelUse::Branch26);
                sink.add_uncond_branch(sink.cur_offset(), sink.cur_offset() + 4, *dest);
                put_u32(sink, enc_b(0));
            }
            Inst::BrTable {
                index,
                tmp,
                targets,
            } => {
                let (default, entries) = targets.split_first().unwrap();
                let index = machreg_to_gpr(*index);
                let n = entries.len() as u32;

                // Bounds check: branch to the default block if `index >=u n`.
                if let Some(imm12) = encode_rotated_imm(n) {
                    put_u32(sink, enc_dp_imm(CmpOp::Cmp.opcode(), 1, 0, index, imm12));
                } else {
                    let tmp = machreg_to_gpr(tmp.to_reg());
                    put_u32(sink, enc_movw(tmp, n & 0xffff));
                    if n >> 16 != 0 {
                        put_u32(sink, enc_movt(tmp, n >> 16));
                    }
                    put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, index, tmp));
                }
                sink.use_label_at_offset(sink.cur_offset(), *default, LabelUse::Branch26);
                put_u32(sink, enc_bcond(Cond::Hs, 0));

                // Jump into the inline table: `pc` reads as this instruction's
                // address + 8, so the table begins one slot after the filler.
                put_u32(sink, 0xe08f_f100 | index); // add pc, pc, index, lsl #2
                put_u32(sink, COND_AL | 0x0320_f000); // nop (filler)
                for target in entries {
                    sink.use_label_at_offset(sink.cur_offset(), *target, LabelUse::Branch26);
                    put_u32(sink, enc_b(0));
                }
            }
            Inst::CondBr {
                cond,
                taken,
                not_taken,
            } => {
                // Conditional branch to `taken`.
                let inverted = enc_bcond(cond.invert(), 0).to_le_bytes();
                sink.use_label_at_offset(sink.cur_offset(), *taken, LabelUse::Branch26);
                sink.add_cond_branch(sink.cur_offset(), sink.cur_offset() + 4, *taken, &inverted);
                put_u32(sink, enc_bcond(*cond, 0));
                // Unconditional branch to `not_taken`.
                sink.use_label_at_offset(sink.cur_offset(), *not_taken, LabelUse::Branch26);
                sink.add_uncond_branch(sink.cur_offset(), sink.cur_offset() + 4, *not_taken);
                put_u32(sink, enc_b(0));
            }

            Inst::Args { .. } | Inst::Rets { .. } => {
                // Pseudo-instructions: no machine code.
            }
        }

        // `BrTable` and calls emit variable-length sequences, so they are
        // exempt from the per-instruction worst-case-size check.
        let variable_length = matches!(
            self,
            Inst::BrTable { .. }
                | Inst::Call { .. }
                | Inst::CallInd { .. }
                | Inst::AtomicRmw { .. }
                | Inst::AtomicCas { .. }
                | Inst::AtomicRmw64 { .. }
                | Inst::AtomicCas64 { .. }
                | Inst::AtomicStore64 { .. }
                | Inst::LoadExtName { .. }
        );
        let end = sink.cur_offset();
        debug_assert!(
            variable_length || (end - start) <= Inst::worst_case_size(),
            "instruction {self:?} exceeded worst-case size"
        );
    }

    fn pretty_print_inst(&self, state: &mut Self::State) -> String {
        self.print_with_state(state)
    }
}

/// Resolve an `AMode` for a VFP access, which only supports an immediate
/// offset. Returns the base GPR encoding and the byte offset.
fn resolve_fpu_amode(mem: &AMode, state: &EmitState) -> (u32, i32) {
    match mem.resolve(state) {
        ResolvedAMode::Imm { base, offset } => (machreg_to_gpr(base), offset),
        ResolvedAMode::Reg { .. } => {
            unimplemented!("arm32: register-offset VFP addressing is not supported")
        }
    }
}

/// Emit the LDAEX/op/STLEX retry loop for an atomic read-modify-write.
///
/// The load zero-extends a sub-word value into `rd`; the store truncates. For
/// add/sub/and/or/xor/nand/xchg the low byte/halfword of the arithmetic is
/// correct regardless of the operand's upper bits, so no extension is needed.
/// The min/max comparisons do extend both inputs to the access width.
fn emit_atomic_rmw(
    sink: &mut MachBuffer<Inst>,
    state: &mut EmitState,
    op: AtomicRmwOp,
    size: AtomicSize,
    rd: u32,
    addr: u32,
    operand: u32,
    tmp1: u32,
    tmp2: u32,
) {
    let loop_lbl = sink.get_label();
    sink.bind_label(loop_lbl, state.ctrl_plane_mut());
    put_u32(sink, enc_load_ex(true, size, rd, addr)); // ldaex{,b,h} rd, [addr]

    // Compute the new value into tmp1.
    let alu = |sink: &mut MachBuffer<Inst>, op: ALUOp| {
        put_u32(sink, enc_dp_reg(op.opcode(), 0, tmp1, rd, operand));
    };
    match op {
        AtomicRmwOp::Add => alu(sink, ALUOp::Add),
        AtomicRmwOp::Sub => alu(sink, ALUOp::Sub),
        AtomicRmwOp::And => alu(sink, ALUOp::And),
        AtomicRmwOp::Or => alu(sink, ALUOp::Orr),
        AtomicRmwOp::Xor => alu(sink, ALUOp::Eor),
        AtomicRmwOp::Nand => {
            alu(sink, ALUOp::And);
            put_u32(sink, enc_dp_reg(0b1111, 0, tmp1, 0, tmp1)); // mvn tmp1, tmp1
        }
        AtomicRmwOp::Xchg => {
            put_u32(sink, enc_dp_reg(0b1101, 0, tmp1, 0, operand)); // mov tmp1, operand
        }
        AtomicRmwOp::Umin | AtomicRmwOp::Umax | AtomicRmwOp::Smin | AtomicRmwOp::Smax => {
            let signed = matches!(op, AtomicRmwOp::Smin | AtomicRmwOp::Smax);
            let cond = match op {
                AtomicRmwOp::Umin => Cond::Lo,
                AtomicRmwOp::Umax => Cond::Hi,
                AtomicRmwOp::Smin => Cond::Lt,
                _ => Cond::Gt,
            };
            if size.is_subword() {
                // Extend both inputs to 32 bits so the comparison is correct,
                // using tmp1/tmp2 as scratch before selecting the result.
                let ext = size.extend_op(signed).template();
                put_u32(sink, enc_op_rd_rm(ext, tmp1, rd)); // ext tmp1, rd
                put_u32(sink, enc_op_rd_rm(ext, tmp2, operand)); // ext tmp2, operand
                put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, tmp1, tmp2));
                put_u32(sink, enc_dp_reg(0b1101, 0, tmp1, 0, operand)); // mov tmp1, operand
                put_u32(sink, enc_mov_cond(cond, tmp1, rd)); // tmp1 = rd if cond
            } else {
                // tmp1 = operand; cmp rd, operand; tmp1 = rd if `cond`.
                put_u32(sink, enc_dp_reg(0b1101, 0, tmp1, 0, operand));
                put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, rd, operand));
                put_u32(sink, enc_mov_cond(cond, tmp1, rd));
            }
        }
    }

    put_u32(sink, enc_store_ex(true, size, tmp2, tmp1, addr)); // stlex{,b,h} tmp2, tmp1, [addr]
    put_u32(sink, enc_dp_imm(CmpOp::Cmp.opcode(), 1, 0, tmp2, 0)); // cmp tmp2, #0
    sink.use_label_at_offset(sink.cur_offset(), loop_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne loop
}

/// Emit the LDAEX/compare/STLEX retry loop for an atomic compare-and-swap.
fn emit_atomic_cas(
    sink: &mut MachBuffer<Inst>,
    state: &mut EmitState,
    size: AtomicSize,
    rd: u32,
    addr: u32,
    expected: u32,
    new: u32,
    tmp: u32,
) {
    let loop_lbl = sink.get_label();
    let done_lbl = sink.get_label();
    sink.bind_label(loop_lbl, state.ctrl_plane_mut());
    put_u32(sink, enc_load_ex(true, size, rd, addr)); // ldaex{,b,h} rd, [addr]
    // `rd` is zero-extended; compare it against a zero-extended `expected` so
    // only the accessed byte/halfword participates.
    if size.is_subword() {
        let ext = size.extend_op(false).template();
        put_u32(sink, enc_op_rd_rm(ext, tmp, expected)); // uxt tmp, expected
        put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, rd, tmp)); // cmp rd, tmp
    } else {
        put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, rd, expected)); // cmp rd, expected
    }
    sink.use_label_at_offset(sink.cur_offset(), done_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne done
    put_u32(sink, enc_store_ex(true, size, tmp, new, addr)); // stlex{,b,h} tmp, new, [addr]
    put_u32(sink, enc_dp_imm(CmpOp::Cmp.opcode(), 1, 0, tmp, 0)); // cmp tmp, #0
    sink.use_label_at_offset(sink.cur_offset(), loop_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne loop
    sink.bind_label(done_lbl, state.ctrl_plane_mut());
}

/// Emit the LDAEXD/op/STLEXD retry loop for a 64-bit atomic read-modify-write.
/// The old value is loaded into (rd_lo, rd_hi) and the new value computed into
/// (nl, nh) before the paired store-exclusive.
fn emit_atomic_rmw64(
    sink: &mut MachBuffer<Inst>,
    state: &mut EmitState,
    op: AtomicRmwOp,
    rd_lo: u32,
    rd_hi: u32,
    addr: u32,
    op_lo: u32,
    op_hi: u32,
    nl: u32,
    nh: u32,
    res: u32,
) {
    debug_assert_eq!(rd_hi, rd_lo + 1, "ldaexd needs a consecutive pair");
    debug_assert_eq!(nh, nl + 1, "stlexd needs a consecutive pair");
    const MOV: u32 = 0b1101;
    const MVN: u32 = 0b1111;
    let loop_lbl = sink.get_label();
    sink.bind_label(loop_lbl, state.ctrl_plane_mut());
    put_u32(sink, enc_load_ex(true, AtomicSize::DWord, rd_lo, addr)); // ldaexd rd_lo,rd_hi,[addr]

    // Compute the new 64-bit value into (nl, nh).
    let dp = |sink: &mut MachBuffer<Inst>, opcode: u32, s: u32, rd: u32, rn: u32, rm: u32| {
        put_u32(sink, enc_dp_reg(opcode, s, rd, rn, rm));
    };
    match op {
        AtomicRmwOp::Add => {
            dp(sink, ALUOp::Add.opcode(), 1, nl, rd_lo, op_lo); // adds nl, rd_lo, op_lo
            dp(sink, ALUOp::Adc.opcode(), 0, nh, rd_hi, op_hi); // adc  nh, rd_hi, op_hi
        }
        AtomicRmwOp::Sub => {
            dp(sink, ALUOp::Sub.opcode(), 1, nl, rd_lo, op_lo); // subs nl, rd_lo, op_lo
            dp(sink, ALUOp::Sbc.opcode(), 0, nh, rd_hi, op_hi); // sbc  nh, rd_hi, op_hi
        }
        AtomicRmwOp::And | AtomicRmwOp::Nand => {
            dp(sink, ALUOp::And.opcode(), 0, nl, rd_lo, op_lo);
            dp(sink, ALUOp::And.opcode(), 0, nh, rd_hi, op_hi);
            if let AtomicRmwOp::Nand = op {
                dp(sink, MVN, 0, nl, 0, nl); // mvn nl, nl
                dp(sink, MVN, 0, nh, 0, nh); // mvn nh, nh
            }
        }
        AtomicRmwOp::Or => {
            dp(sink, ALUOp::Orr.opcode(), 0, nl, rd_lo, op_lo);
            dp(sink, ALUOp::Orr.opcode(), 0, nh, rd_hi, op_hi);
        }
        AtomicRmwOp::Xor => {
            dp(sink, ALUOp::Eor.opcode(), 0, nl, rd_lo, op_lo);
            dp(sink, ALUOp::Eor.opcode(), 0, nh, rd_hi, op_hi);
        }
        AtomicRmwOp::Xchg => {
            dp(sink, MOV, 0, nl, 0, op_lo); // mov nl, op_lo
            dp(sink, MOV, 0, nh, 0, op_hi); // mov nh, op_hi
        }
        AtomicRmwOp::Umin | AtomicRmwOp::Umax | AtomicRmwOp::Smin | AtomicRmwOp::Smax => {
            // Set flags for `old - operand` (64-bit) via subs/sbcs, using
            // (nl, nh) as scratch. The chosen condition is Z-independent (the
            // sbcs Z flag reflects only the high word), so equality never
            // affects the result — which is correct, since min/max of equal
            // values is that value either way.
            let cond = match op {
                AtomicRmwOp::Umin => Cond::Lo, // old <u operand
                AtomicRmwOp::Umax => Cond::Hs, // old >=u operand
                AtomicRmwOp::Smin => Cond::Lt, // old <s operand
                _ => Cond::Ge,                 // old >=s operand
            };
            dp(sink, ALUOp::Sub.opcode(), 1, nl, rd_lo, op_lo); // subs nl, rd_lo, op_lo
            dp(sink, ALUOp::Sbc.opcode(), 1, nh, rd_hi, op_hi); // sbcs nh, rd_hi, op_hi
            dp(sink, MOV, 0, nl, 0, op_lo); // nl = operand (default)
            dp(sink, MOV, 0, nh, 0, op_hi);
            put_u32(sink, enc_mov_cond(cond, nl, rd_lo)); // nl = rd_lo if cond
            put_u32(sink, enc_mov_cond(cond, nh, rd_hi)); // nh = rd_hi if cond
        }
    }

    put_u32(sink, enc_store_ex(true, AtomicSize::DWord, res, nl, addr)); // stlexd res, nl,nh,[addr]
    put_u32(sink, enc_dp_imm(CmpOp::Cmp.opcode(), 1, 0, res, 0)); // cmp res, #0
    sink.use_label_at_offset(sink.cur_offset(), loop_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne loop
}

/// Emit the LDAEXD/compare/STLEXD retry loop for a 64-bit compare-and-swap.
fn emit_atomic_cas64(
    sink: &mut MachBuffer<Inst>,
    state: &mut EmitState,
    rd_lo: u32,
    rd_hi: u32,
    addr: u32,
    exp_lo: u32,
    exp_hi: u32,
    new_lo: u32,
    new_hi: u32,
    res: u32,
) {
    debug_assert_eq!(rd_hi, rd_lo + 1, "ldaexd needs a consecutive pair");
    debug_assert_eq!(new_hi, new_lo + 1, "stlexd needs a consecutive pair");
    let loop_lbl = sink.get_label();
    let done_lbl = sink.get_label();
    sink.bind_label(loop_lbl, state.ctrl_plane_mut());
    put_u32(sink, enc_load_ex(true, AtomicSize::DWord, rd_lo, addr)); // ldaexd rd_lo,rd_hi,[addr]
    // Both halves must match `expected`.
    put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, rd_lo, exp_lo)); // cmp rd_lo, exp_lo
    sink.use_label_at_offset(sink.cur_offset(), done_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne done
    put_u32(sink, enc_dp_reg(CmpOp::Cmp.opcode(), 1, 0, rd_hi, exp_hi)); // cmp rd_hi, exp_hi
    sink.use_label_at_offset(sink.cur_offset(), done_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne done
    put_u32(
        sink,
        enc_store_ex(true, AtomicSize::DWord, res, new_lo, addr),
    ); // stlexd res,new_lo,new_hi
    put_u32(sink, enc_dp_imm(CmpOp::Cmp.opcode(), 1, 0, res, 0)); // cmp res, #0
    sink.use_label_at_offset(sink.cur_offset(), loop_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne loop
    sink.bind_label(done_lbl, state.ctrl_plane_mut());
}

/// Emit the LDAEXD/STLEXD retry loop for a 64-bit atomic store. The load is
/// required to arm the exclusive monitor; its result is discarded.
fn emit_atomic_store64(
    sink: &mut MachBuffer<Inst>,
    state: &mut EmitState,
    addr: u32,
    src_lo: u32,
    src_hi: u32,
    scratch_lo: u32,
    scratch_hi: u32,
    res: u32,
) {
    debug_assert_eq!(
        scratch_hi,
        scratch_lo + 1,
        "ldaexd needs a consecutive pair"
    );
    debug_assert_eq!(src_hi, src_lo + 1, "stlexd needs a consecutive pair");
    let loop_lbl = sink.get_label();
    sink.bind_label(loop_lbl, state.ctrl_plane_mut());
    put_u32(sink, enc_load_ex(true, AtomicSize::DWord, scratch_lo, addr)); // ldaexd scratch,[addr]
    put_u32(
        sink,
        enc_store_ex(true, AtomicSize::DWord, res, src_lo, addr),
    ); // stlexd res, src,[addr]
    put_u32(sink, enc_dp_imm(CmpOp::Cmp.opcode(), 1, 0, res, 0)); // cmp res, #0
    sink.use_label_at_offset(sink.cur_offset(), loop_lbl, LabelUse::Branch26);
    put_u32(sink, enc_bcond(Cond::Ne, 0)); // bne loop
}

enum LoadStore {
    Load(LoadKind),
    Store(StoreKind),
}

fn emit_load_store(
    sink: &mut MachBuffer<Inst>,
    rt: u32,
    mem: &AMode,
    state: &EmitState,
    op: LoadStore,
) {
    match mem.resolve(state) {
        ResolvedAMode::Imm { base, offset } => {
            let base = machreg_to_gpr(base);
            let word = match op {
                LoadStore::Load(LoadKind::Word) => enc_ldr_str_imm(true, false, rt, base, offset),
                LoadStore::Load(LoadKind::UByte) => enc_ldr_str_imm(true, true, rt, base, offset),
                LoadStore::Load(LoadKind::SByte) => enc_ldrh_strh_imm(true, 1, 0, rt, base, offset),
                LoadStore::Load(LoadKind::UHalf) => enc_ldrh_strh_imm(true, 0, 1, rt, base, offset),
                LoadStore::Load(LoadKind::SHalf) => enc_ldrh_strh_imm(true, 1, 1, rt, base, offset),
                LoadStore::Store(StoreKind::Word) => {
                    enc_ldr_str_imm(false, false, rt, base, offset)
                }
                LoadStore::Store(StoreKind::Byte) => enc_ldr_str_imm(false, true, rt, base, offset),
                LoadStore::Store(StoreKind::Half) => {
                    enc_ldrh_strh_imm(false, 0, 1, rt, base, offset)
                }
            };
            put_u32(sink, word);
        }
        ResolvedAMode::Reg { base, index } => {
            let base = machreg_to_gpr(base);
            let index = machreg_to_gpr(index);
            let word = match op {
                LoadStore::Load(LoadKind::Word) => enc_ldr_str_reg(true, false, rt, base, index),
                LoadStore::Load(LoadKind::UByte) => enc_ldr_str_reg(true, true, rt, base, index),
                LoadStore::Store(StoreKind::Word) => enc_ldr_str_reg(false, false, rt, base, index),
                LoadStore::Store(StoreKind::Byte) => enc_ldr_str_reg(false, true, rt, base, index),
                _ => unimplemented!("arm32: register-offset sub-word access not yet implemented"),
            };
            put_u32(sink, word);
        }
    }
}
