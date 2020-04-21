//! AArch64 ISA: binary code emission.

use crate::binemit::{CodeOffset, Reloc};
use crate::ir::constant::ConstantData;
use crate::ir::types::*;
use crate::ir::TrapCode;
use crate::isa::aarch64::{inst::regs::PINNED_REG, inst::*};

use regalloc::{Reg, RegClass, Writable};

use alloc::vec::Vec;
use core::convert::TryFrom;

/// Memory label/reference finalization: convert a MemLabel to a PC-relative
/// offset, possibly emitting relocation(s) as necessary.
pub fn memlabel_finalize(_insn_off: CodeOffset, label: &MemLabel) -> i32 {
    match label {
        &MemLabel::PCRel(rel) => rel,
    }
}

/// Memory addressing mode finalization: convert "special" modes (e.g.,
/// generic arbitrary stack offset) into real addressing modes, possibly by
/// emitting some helper instructions that come immediately before the use
/// of this amode.
pub fn mem_finalize(insn_off: CodeOffset, mem: &MemArg) -> (Vec<Inst>, MemArg) {
    match mem {
        &MemArg::SPOffset(off) | &MemArg::FPOffset(off) => {
            let basereg = match mem {
                &MemArg::SPOffset(..) => stack_reg(),
                &MemArg::FPOffset(..) => fp_reg(),
                _ => unreachable!(),
            };
            if let Some(simm9) = SImm9::maybe_from_i64(off) {
                let mem = MemArg::Unscaled(basereg, simm9);
                (vec![], mem)
            } else {
                let tmp = writable_spilltmp_reg();
                let mut const_insts = Inst::load_constant(tmp, off as u64);
                let add_inst = Inst::AluRRR {
                    alu_op: ALUOp::Add64,
                    rd: tmp,
                    rn: tmp.to_reg(),
                    rm: basereg,
                };
                const_insts.push(add_inst);
                (const_insts.to_vec(), MemArg::reg(tmp.to_reg()))
            }
        }
        &MemArg::Label(ref label) => {
            let off = memlabel_finalize(insn_off, label);
            (vec![], MemArg::Label(MemLabel::PCRel(off)))
        }
        _ => (vec![], mem.clone()),
    }
}

/// Helper: get a ConstantData from a u64.
pub fn u64_constant(bits: u64) -> ConstantData {
    let data = bits.to_le_bytes();
    ConstantData::from(&data[..])
}

//=============================================================================
// Instructions and subcomponents: emission

fn machreg_to_gpr(m: Reg) -> u32 {
    assert!(m.get_class() == RegClass::I64);
    u32::try_from(m.to_real_reg().get_hw_encoding()).unwrap()
}

fn machreg_to_vec(m: Reg) -> u32 {
    assert!(m.get_class() == RegClass::V128);
    u32::try_from(m.to_real_reg().get_hw_encoding()).unwrap()
}

fn machreg_to_gpr_or_vec(m: Reg) -> u32 {
    u32::try_from(m.to_real_reg().get_hw_encoding()).unwrap()
}

fn enc_arith_rrr(bits_31_21: u32, bits_15_10: u32, rd: Writable<Reg>, rn: Reg, rm: Reg) -> u32 {
    (bits_31_21 << 21)
        | (bits_15_10 << 10)
        | machreg_to_gpr(rd.to_reg())
        | (machreg_to_gpr(rn) << 5)
        | (machreg_to_gpr(rm) << 16)
}

fn enc_arith_rr_imm12(
    bits_31_24: u32,
    immshift: u32,
    imm12: u32,
    rn: Reg,
    rd: Writable<Reg>,
) -> u32 {
    (bits_31_24 << 24)
        | (immshift << 22)
        | (imm12 << 10)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr(rd.to_reg())
}

fn enc_arith_rr_imml(bits_31_23: u32, imm_bits: u32, rn: Reg, rd: Writable<Reg>) -> u32 {
    (bits_31_23 << 23) | (imm_bits << 10) | (machreg_to_gpr(rn) << 5) | machreg_to_gpr(rd.to_reg())
}

fn enc_arith_rrrr(top11: u32, rm: Reg, bit15: u32, ra: Reg, rn: Reg, rd: Writable<Reg>) -> u32 {
    (top11 << 21)
        | (machreg_to_gpr(rm) << 16)
        | (bit15 << 15)
        | (machreg_to_gpr(ra) << 10)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr(rd.to_reg())
}

fn enc_jump26(op_31_26: u32, off_26_0: u32) -> u32 {
    assert!(off_26_0 < (1 << 26));
    (op_31_26 << 26) | off_26_0
}

fn enc_cmpbr(op_31_24: u32, off_18_0: u32, reg: Reg) -> u32 {
    assert!(off_18_0 < (1 << 19));
    (op_31_24 << 24) | (off_18_0 << 5) | machreg_to_gpr(reg)
}

fn enc_cbr(op_31_24: u32, off_18_0: u32, op_4: u32, cond: u32) -> u32 {
    assert!(off_18_0 < (1 << 19));
    assert!(cond < (1 << 4));
    (op_31_24 << 24) | (off_18_0 << 5) | (op_4 << 4) | cond
}

const MOVE_WIDE_FIXED: u32 = 0x92800000;

#[repr(u32)]
enum MoveWideOpcode {
    MOVN = 0b00,
    MOVZ = 0b10,
    MOVK = 0b11,
}

fn enc_move_wide(op: MoveWideOpcode, rd: Writable<Reg>, imm: MoveWideConst) -> u32 {
    assert!(imm.shift <= 0b11);
    MOVE_WIDE_FIXED
        | (op as u32) << 29
        | u32::from(imm.shift) << 21
        | u32::from(imm.bits) << 5
        | machreg_to_gpr(rd.to_reg())
}

fn enc_ldst_pair(op_31_22: u32, simm7: SImm7Scaled, rn: Reg, rt: Reg, rt2: Reg) -> u32 {
    (op_31_22 << 22)
        | (simm7.bits() << 15)
        | (machreg_to_gpr(rt2) << 10)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr(rt)
}

fn enc_ldst_simm9(op_31_22: u32, simm9: SImm9, op_11_10: u32, rn: Reg, rd: Reg) -> u32 {
    (op_31_22 << 22)
        | (simm9.bits() << 12)
        | (op_11_10 << 10)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr_or_vec(rd)
}

fn enc_ldst_uimm12(op_31_22: u32, uimm12: UImm12Scaled, rn: Reg, rd: Reg) -> u32 {
    (op_31_22 << 22)
        | (0b1 << 24)
        | (uimm12.bits() << 10)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr_or_vec(rd)
}

fn enc_ldst_reg(
    op_31_22: u32,
    rn: Reg,
    rm: Reg,
    s_bit: bool,
    extendop: Option<ExtendOp>,
    rd: Reg,
) -> u32 {
    let s_bit = if s_bit { 1 } else { 0 };
    let extend_bits = match extendop {
        Some(ExtendOp::UXTW) => 0b010,
        Some(ExtendOp::SXTW) => 0b110,
        Some(ExtendOp::SXTX) => 0b111,
        None => 0b011, // LSL
        _ => panic!("bad extend mode for ld/st MemArg"),
    };
    (op_31_22 << 22)
        | (1 << 21)
        | (machreg_to_gpr(rm) << 16)
        | (extend_bits << 13)
        | (s_bit << 12)
        | (0b10 << 10)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr_or_vec(rd)
}

fn enc_ldst_imm19(op_31_24: u32, imm19: u32, rd: Reg) -> u32 {
    (op_31_24 << 24) | (imm19 << 5) | machreg_to_gpr_or_vec(rd)
}

fn enc_extend(top22: u32, rd: Writable<Reg>, rn: Reg) -> u32 {
    (top22 << 10) | (machreg_to_gpr(rn) << 5) | machreg_to_gpr(rd.to_reg())
}

fn enc_vec_rrr(top11: u32, rm: Reg, bit15_10: u32, rn: Reg, rd: Writable<Reg>) -> u32 {
    (top11 << 21)
        | (machreg_to_vec(rm) << 16)
        | (bit15_10 << 10)
        | (machreg_to_vec(rn) << 5)
        | machreg_to_vec(rd.to_reg())
}

fn enc_bit_rr(size: u32, opcode2: u32, opcode1: u32, rn: Reg, rd: Writable<Reg>) -> u32 {
    (0b01011010110 << 21)
        | size << 31
        | opcode2 << 16
        | opcode1 << 10
        | machreg_to_gpr(rn) << 5
        | machreg_to_gpr(rd.to_reg())
}

fn enc_br(rn: Reg) -> u32 {
    0b1101011_0000_11111_000000_00000_00000 | (machreg_to_gpr(rn) << 5)
}

fn enc_adr(off: i32, rd: Writable<Reg>) -> u32 {
    let off = u32::try_from(off).unwrap();
    let immlo = off & 3;
    let immhi = (off >> 2) & ((1 << 19) - 1);
    (0b00010000 << 24) | (immlo << 29) | (immhi << 5) | machreg_to_gpr(rd.to_reg())
}

fn enc_csel(rd: Writable<Reg>, rn: Reg, rm: Reg, cond: Cond) -> u32 {
    0b100_11010100_00000_0000_00_00000_00000
        | (machreg_to_gpr(rm) << 16)
        | (machreg_to_gpr(rn) << 5)
        | machreg_to_gpr(rd.to_reg())
        | (cond.bits() << 12)
}

fn enc_fcsel(rd: Writable<Reg>, rn: Reg, rm: Reg, cond: Cond, size: InstSize) -> u32 {
    let ty_bit = if size.is32() { 0 } else { 1 };
    0b000_11110_00_1_00000_0000_11_00000_00000
        | (machreg_to_vec(rm) << 16)
        | (machreg_to_vec(rn) << 5)
        | machreg_to_vec(rd.to_reg())
        | (cond.bits() << 12)
        | (ty_bit << 22)
}

fn enc_cset(rd: Writable<Reg>, cond: Cond) -> u32 {
    0b100_11010100_11111_0000_01_11111_00000
        | machreg_to_gpr(rd.to_reg())
        | (cond.invert().bits() << 12)
}

fn enc_vecmov(is_16b: bool, rd: Writable<Reg>, rn: Reg) -> u32 {
    debug_assert!(!is_16b); // to be supported later.
    0b00001110_101_00000_00011_1_00000_00000
        | machreg_to_vec(rd.to_reg())
        | (machreg_to_vec(rn) << 16)
        | (machreg_to_vec(rn) << 5)
}

fn enc_fpurr(top22: u32, rd: Writable<Reg>, rn: Reg) -> u32 {
    (top22 << 10) | (machreg_to_vec(rn) << 5) | machreg_to_vec(rd.to_reg())
}

fn enc_fpurrr(top22: u32, rd: Writable<Reg>, rn: Reg, rm: Reg) -> u32 {
    (top22 << 10)
        | (machreg_to_vec(rm) << 16)
        | (machreg_to_vec(rn) << 5)
        | machreg_to_vec(rd.to_reg())
}

fn enc_fpurrrr(top17: u32, rd: Writable<Reg>, rn: Reg, rm: Reg, ra: Reg) -> u32 {
    (top17 << 15)
        | (machreg_to_vec(rm) << 16)
        | (machreg_to_vec(ra) << 10)
        | (machreg_to_vec(rn) << 5)
        | machreg_to_vec(rd.to_reg())
}

fn enc_fcmp(size: InstSize, rn: Reg, rm: Reg) -> u32 {
    let bits = if size.is32() {
        0b000_11110_00_1_00000_00_1000_00000_00000
    } else {
        0b000_11110_01_1_00000_00_1000_00000_00000
    };
    bits | (machreg_to_vec(rm) << 16) | (machreg_to_vec(rn) << 5)
}

fn enc_fputoint(top16: u32, rd: Writable<Reg>, rn: Reg) -> u32 {
    (top16 << 16) | (machreg_to_vec(rn) << 5) | machreg_to_gpr(rd.to_reg())
}

fn enc_inttofpu(top16: u32, rd: Writable<Reg>, rn: Reg) -> u32 {
    (top16 << 16) | (machreg_to_gpr(rn) << 5) | machreg_to_vec(rd.to_reg())
}

fn enc_fround(top22: u32, rd: Writable<Reg>, rn: Reg) -> u32 {
    (top22 << 10) | (machreg_to_vec(rn) << 5) | machreg_to_vec(rd.to_reg())
}

impl<O: MachSectionOutput> MachInstEmit<O> for Inst {
    fn emit(&self, sink: &mut O) {
        match self {
            &Inst::AluRRR { alu_op, rd, rn, rm } => {
                let top11 = match alu_op {
                    ALUOp::Add32 => 0b00001011_000,
                    ALUOp::Add64 => 0b10001011_000,
                    ALUOp::Sub32 => 0b01001011_000,
                    ALUOp::Sub64 => 0b11001011_000,
                    ALUOp::Orr32 => 0b00101010_000,
                    ALUOp::Orr64 => 0b10101010_000,
                    ALUOp::And32 => 0b00001010_000,
                    ALUOp::And64 => 0b10001010_000,
                    ALUOp::Eor32 => 0b01001010_000,
                    ALUOp::Eor64 => 0b11001010_000,
                    ALUOp::OrrNot32 => 0b00101010_001,
                    ALUOp::OrrNot64 => 0b10101010_001,
                    ALUOp::AndNot32 => 0b00001010_001,
                    ALUOp::AndNot64 => 0b10001010_001,
                    ALUOp::EorNot32 => 0b01001010_001,
                    ALUOp::EorNot64 => 0b11001010_001,
                    ALUOp::AddS32 => 0b00101011_000,
                    ALUOp::AddS64 => 0b10101011_000,
                    ALUOp::SubS32 => 0b01101011_000,
                    ALUOp::SubS64 => 0b11101011_000,
                    ALUOp::SDiv64 => 0b10011010_110,
                    ALUOp::UDiv64 => 0b10011010_110,
                    ALUOp::RotR32 | ALUOp::Lsr32 | ALUOp::Asr32 | ALUOp::Lsl32 => 0b00011010_110,
                    ALUOp::RotR64 | ALUOp::Lsr64 | ALUOp::Asr64 | ALUOp::Lsl64 => 0b10011010_110,

                    ALUOp::MAdd32
                    | ALUOp::MAdd64
                    | ALUOp::MSub32
                    | ALUOp::MSub64
                    | ALUOp::SMulH
                    | ALUOp::UMulH => {
                        //// RRRR ops.
                        panic!("Bad ALUOp {:?} in RRR form!", alu_op);
                    }
                };
                let bit15_10 = match alu_op {
                    ALUOp::SDiv64 => 0b000011,
                    ALUOp::UDiv64 => 0b000010,
                    ALUOp::RotR32 | ALUOp::RotR64 => 0b001011,
                    ALUOp::Lsr32 | ALUOp::Lsr64 => 0b001001,
                    ALUOp::Asr32 | ALUOp::Asr64 => 0b001010,
                    ALUOp::Lsl32 | ALUOp::Lsl64 => 0b001000,
                    _ => 0b000000,
                };
                assert_ne!(writable_stack_reg(), rd);
                sink.put4(enc_arith_rrr(top11, bit15_10, rd, rn, rm));
            }
            &Inst::AluRRRR {
                alu_op,
                rd,
                rm,
                rn,
                ra,
            } => {
                let (top11, bit15) = match alu_op {
                    ALUOp::MAdd32 => (0b0_00_11011_000, 0),
                    ALUOp::MSub32 => (0b0_00_11011_000, 1),
                    ALUOp::MAdd64 => (0b1_00_11011_000, 0),
                    ALUOp::MSub64 => (0b1_00_11011_000, 1),
                    ALUOp::SMulH => (0b1_00_11011_010, 0),
                    ALUOp::UMulH => (0b1_00_11011_110, 0),
                    _ => unimplemented!("{:?}", alu_op),
                };
                sink.put4(enc_arith_rrrr(top11, rm, bit15, ra, rn, rd));
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rn,
                ref imm12,
            } => {
                let top8 = match alu_op {
                    ALUOp::Add32 => 0b000_10001,
                    ALUOp::Add64 => 0b100_10001,
                    ALUOp::Sub32 => 0b010_10001,
                    ALUOp::Sub64 => 0b110_10001,
                    ALUOp::AddS32 => 0b001_10001,
                    ALUOp::AddS64 => 0b101_10001,
                    ALUOp::SubS32 => 0b011_10001,
                    ALUOp::SubS64 => 0b111_10001,
                    _ => unimplemented!("{:?}", alu_op),
                };
                sink.put4(enc_arith_rr_imm12(
                    top8,
                    imm12.shift_bits(),
                    imm12.imm_bits(),
                    rn,
                    rd,
                ));
            }
            &Inst::AluRRImmLogic {
                alu_op,
                rd,
                rn,
                ref imml,
            } => {
                let (top9, inv) = match alu_op {
                    ALUOp::Orr32 => (0b001_100100, false),
                    ALUOp::Orr64 => (0b101_100100, false),
                    ALUOp::And32 => (0b000_100100, false),
                    ALUOp::And64 => (0b100_100100, false),
                    ALUOp::Eor32 => (0b010_100100, false),
                    ALUOp::Eor64 => (0b110_100100, false),
                    ALUOp::OrrNot32 => (0b001_100100, true),
                    ALUOp::OrrNot64 => (0b101_100100, true),
                    ALUOp::AndNot32 => (0b000_100100, true),
                    ALUOp::AndNot64 => (0b100_100100, true),
                    ALUOp::EorNot32 => (0b010_100100, true),
                    ALUOp::EorNot64 => (0b110_100100, true),
                    _ => unimplemented!("{:?}", alu_op),
                };
                let imml = if inv { imml.invert() } else { imml.clone() };
                sink.put4(enc_arith_rr_imml(top9, imml.enc_bits(), rn, rd));
            }

            &Inst::AluRRImmShift {
                alu_op,
                rd,
                rn,
                ref immshift,
            } => {
                let amt = immshift.value();
                let (top10, immr, imms) = match alu_op {
                    ALUOp::RotR32 => (0b0001001110, machreg_to_gpr(rn), u32::from(amt)),
                    ALUOp::RotR64 => (0b1001001111, machreg_to_gpr(rn), u32::from(amt)),
                    ALUOp::Lsr32 => (0b0101001100, u32::from(amt), 0b011111),
                    ALUOp::Lsr64 => (0b1101001101, u32::from(amt), 0b111111),
                    ALUOp::Asr32 => (0b0001001100, u32::from(amt), 0b011111),
                    ALUOp::Asr64 => (0b1001001101, u32::from(amt), 0b111111),
                    ALUOp::Lsl32 => (0b0101001100, u32::from(32 - amt), u32::from(31 - amt)),
                    ALUOp::Lsl64 => (0b1101001101, u32::from(64 - amt), u32::from(63 - amt)),
                    _ => unimplemented!("{:?}", alu_op),
                };
                sink.put4(
                    (top10 << 22)
                        | (immr << 16)
                        | (imms << 10)
                        | (machreg_to_gpr(rn) << 5)
                        | machreg_to_gpr(rd.to_reg()),
                );
            }

            &Inst::AluRRRShift {
                alu_op,
                rd,
                rn,
                rm,
                ref shiftop,
            } => {
                let top11: u32 = match alu_op {
                    ALUOp::Add32 => 0b000_01011000,
                    ALUOp::Add64 => 0b100_01011000,
                    ALUOp::AddS32 => 0b001_01011000,
                    ALUOp::AddS64 => 0b101_01011000,
                    ALUOp::Sub32 => 0b010_01011000,
                    ALUOp::Sub64 => 0b110_01011000,
                    ALUOp::SubS32 => 0b011_01011000,
                    ALUOp::SubS64 => 0b111_01011000,
                    ALUOp::Orr32 => 0b001_01010000,
                    ALUOp::Orr64 => 0b101_01010000,
                    ALUOp::And32 => 0b000_01010000,
                    ALUOp::And64 => 0b100_01010000,
                    ALUOp::Eor32 => 0b010_01010000,
                    ALUOp::Eor64 => 0b110_01010000,
                    ALUOp::OrrNot32 => 0b001_01010001,
                    ALUOp::OrrNot64 => 0b101_01010001,
                    ALUOp::EorNot32 => 0b010_01010001,
                    ALUOp::EorNot64 => 0b110_01010001,
                    ALUOp::AndNot32 => 0b000_01010001,
                    ALUOp::AndNot64 => 0b100_01010001,
                    _ => unimplemented!("{:?}", alu_op),
                };
                let top11 = top11 | (u32::from(shiftop.op().bits()) << 1);
                let bits_15_10 = u32::from(shiftop.amt().value());
                sink.put4(enc_arith_rrr(top11, bits_15_10, rd, rn, rm));
            }

            &Inst::AluRRRExtend {
                alu_op,
                rd,
                rn,
                rm,
                extendop,
            } => {
                let top11: u32 = match alu_op {
                    ALUOp::Add32 => 0b00001011001,
                    ALUOp::Add64 => 0b10001011001,
                    ALUOp::Sub32 => 0b01001011001,
                    ALUOp::Sub64 => 0b11001011001,
                    ALUOp::AddS32 => 0b00101011001,
                    ALUOp::AddS64 => 0b10101011001,
                    ALUOp::SubS32 => 0b01101011001,
                    ALUOp::SubS64 => 0b11101011001,
                    _ => unimplemented!("{:?}", alu_op),
                };
                let bits_15_10 = u32::from(extendop.bits()) << 3;
                sink.put4(enc_arith_rrr(top11, bits_15_10, rd, rn, rm));
            }

            &Inst::BitRR { op, rd, rn, .. } => {
                let size = if op.inst_size().is32() { 0b0 } else { 0b1 };
                let (op1, op2) = match op {
                    BitOp::RBit32 | BitOp::RBit64 => (0b00000, 0b000000),
                    BitOp::Clz32 | BitOp::Clz64 => (0b00000, 0b000100),
                    BitOp::Cls32 | BitOp::Cls64 => (0b00000, 0b000101),
                };
                sink.put4(enc_bit_rr(size, op1, op2, rn, rd))
            }

            &Inst::ULoad8 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::SLoad8 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::ULoad16 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::SLoad16 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::ULoad32 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::SLoad32 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::ULoad64 {
                rd,
                ref mem,
                srcloc,
                ..
            }
            | &Inst::FpuLoad32 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::FpuLoad64 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::FpuLoad128 {
                rd,
                ref mem,
                srcloc,
            } => {
                let (mem_insts, mem) = mem_finalize(sink.cur_offset_from_start(), mem);

                for inst in mem_insts.into_iter() {
                    inst.emit(sink);
                }

                // ldst encoding helpers take Reg, not Writable<Reg>.
                let rd = rd.to_reg();

                // This is the base opcode (top 10 bits) for the "unscaled
                // immediate" form (Unscaled). Other addressing modes will OR in
                // other values for bits 24/25 (bits 1/2 of this constant).
                let op = match self {
                    &Inst::ULoad8 { .. } => 0b0011100001,
                    &Inst::SLoad8 { .. } => 0b0011100010,
                    &Inst::ULoad16 { .. } => 0b0111100001,
                    &Inst::SLoad16 { .. } => 0b0111100010,
                    &Inst::ULoad32 { .. } => 0b1011100001,
                    &Inst::SLoad32 { .. } => 0b1011100010,
                    &Inst::ULoad64 { .. } => 0b1111100001,
                    &Inst::FpuLoad32 { .. } => 0b1011110001,
                    &Inst::FpuLoad64 { .. } => 0b1111110001,
                    &Inst::FpuLoad128 { .. } => 0b0011110011,
                    _ => unreachable!(),
                };

                if let Some(srcloc) = srcloc {
                    // Register the offset at which the actual load instruction starts.
                    sink.add_trap(srcloc, TrapCode::OutOfBounds);
                }

                match &mem {
                    &MemArg::Unscaled(reg, simm9) => {
                        sink.put4(enc_ldst_simm9(op, simm9, 0b00, reg, rd));
                    }
                    &MemArg::UnsignedOffset(reg, uimm12scaled) => {
                        sink.put4(enc_ldst_uimm12(op, uimm12scaled, reg, rd));
                    }
                    &MemArg::RegReg(r1, r2) => {
                        sink.put4(enc_ldst_reg(
                            op, r1, r2, /* scaled = */ false, /* extendop = */ None, rd,
                        ));
                    }
                    &MemArg::RegScaled(r1, r2, ty) | &MemArg::RegScaledExtended(r1, r2, ty, _) => {
                        match (ty, self) {
                            (I8, &Inst::ULoad8 { .. }) => {}
                            (I8, &Inst::SLoad8 { .. }) => {}
                            (I16, &Inst::ULoad16 { .. }) => {}
                            (I16, &Inst::SLoad16 { .. }) => {}
                            (I32, &Inst::ULoad32 { .. }) => {}
                            (I32, &Inst::SLoad32 { .. }) => {}
                            (I64, &Inst::ULoad64 { .. }) => {}
                            (F32, &Inst::FpuLoad32 { .. }) => {}
                            (F64, &Inst::FpuLoad64 { .. }) => {}
                            (I128, &Inst::FpuLoad128 { .. }) => {}
                            _ => panic!("Mismatching reg-scaling type in MemArg"),
                        }
                        let extendop = match &mem {
                            &MemArg::RegScaled(..) => None,
                            &MemArg::RegScaledExtended(_, _, _, op) => Some(op),
                            _ => unreachable!(),
                        };
                        sink.put4(enc_ldst_reg(
                            op, r1, r2, /* scaled = */ true, extendop, rd,
                        ));
                    }
                    &MemArg::Label(ref label) => {
                        let offset = match label {
                            // cast i32 to u32 (two's-complement)
                            &MemLabel::PCRel(off) => off as u32,
                        } / 4;
                        assert!(offset < (1 << 19));
                        match self {
                            &Inst::ULoad32 { .. } => {
                                sink.put4(enc_ldst_imm19(0b00011000, offset, rd));
                            }
                            &Inst::SLoad32 { .. } => {
                                sink.put4(enc_ldst_imm19(0b10011000, offset, rd));
                            }
                            &Inst::FpuLoad32 { .. } => {
                                sink.put4(enc_ldst_imm19(0b00011100, offset, rd));
                            }
                            &Inst::ULoad64 { .. } => {
                                sink.put4(enc_ldst_imm19(0b01011000, offset, rd));
                            }
                            &Inst::FpuLoad64 { .. } => {
                                sink.put4(enc_ldst_imm19(0b01011100, offset, rd));
                            }
                            &Inst::FpuLoad128 { .. } => {
                                sink.put4(enc_ldst_imm19(0b10011100, offset, rd));
                            }
                            _ => panic!("Unspported size for LDR from constant pool!"),
                        }
                    }
                    &MemArg::PreIndexed(reg, simm9) => {
                        sink.put4(enc_ldst_simm9(op, simm9, 0b11, reg.to_reg(), rd));
                    }
                    &MemArg::PostIndexed(reg, simm9) => {
                        sink.put4(enc_ldst_simm9(op, simm9, 0b01, reg.to_reg(), rd));
                    }
                    // Eliminated by `mem_finalize()` above.
                    &MemArg::SPOffset(..) | &MemArg::FPOffset(..) => {
                        panic!("Should not see stack-offset here!")
                    }
                }
            }

            &Inst::Store8 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::Store16 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::Store32 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::Store64 {
                rd,
                ref mem,
                srcloc,
                ..
            }
            | &Inst::FpuStore32 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::FpuStore64 {
                rd,
                ref mem,
                srcloc,
            }
            | &Inst::FpuStore128 {
                rd,
                ref mem,
                srcloc,
            } => {
                let (mem_insts, mem) = mem_finalize(sink.cur_offset_from_start(), mem);

                for inst in mem_insts.into_iter() {
                    inst.emit(sink);
                }

                let op = match self {
                    &Inst::Store8 { .. } => 0b0011100000,
                    &Inst::Store16 { .. } => 0b0111100000,
                    &Inst::Store32 { .. } => 0b1011100000,
                    &Inst::Store64 { .. } => 0b1111100000,
                    &Inst::FpuStore32 { .. } => 0b1011110000,
                    &Inst::FpuStore64 { .. } => 0b1111110000,
                    &Inst::FpuStore128 { .. } => 0b0011110010,
                    _ => unreachable!(),
                };

                if let Some(srcloc) = srcloc {
                    // Register the offset at which the actual load instruction starts.
                    sink.add_trap(srcloc, TrapCode::OutOfBounds);
                }

                match &mem {
                    &MemArg::Unscaled(reg, simm9) => {
                        sink.put4(enc_ldst_simm9(op, simm9, 0b00, reg, rd));
                    }
                    &MemArg::UnsignedOffset(reg, uimm12scaled) => {
                        sink.put4(enc_ldst_uimm12(op, uimm12scaled, reg, rd));
                    }
                    &MemArg::RegReg(r1, r2) => {
                        sink.put4(enc_ldst_reg(
                            op, r1, r2, /* scaled = */ false, /* extendop = */ None, rd,
                        ));
                    }
                    &MemArg::RegScaled(r1, r2, _ty)
                    | &MemArg::RegScaledExtended(r1, r2, _ty, _) => {
                        let extendop = match &mem {
                            &MemArg::RegScaled(..) => None,
                            &MemArg::RegScaledExtended(_, _, _, op) => Some(op),
                            _ => unreachable!(),
                        };
                        sink.put4(enc_ldst_reg(
                            op, r1, r2, /* scaled = */ true, extendop, rd,
                        ));
                    }
                    &MemArg::Label(..) => {
                        panic!("Store to a MemLabel not implemented!");
                    }
                    &MemArg::PreIndexed(reg, simm9) => {
                        sink.put4(enc_ldst_simm9(op, simm9, 0b11, reg.to_reg(), rd));
                    }
                    &MemArg::PostIndexed(reg, simm9) => {
                        sink.put4(enc_ldst_simm9(op, simm9, 0b01, reg.to_reg(), rd));
                    }
                    // Eliminated by `mem_finalize()` above.
                    &MemArg::SPOffset(..) | &MemArg::FPOffset(..) => {
                        panic!("Should not see stack-offset here!")
                    }
                }
            }

            &Inst::StoreP64 { rt, rt2, ref mem } => match mem {
                &PairMemArg::SignedOffset(reg, simm7) => {
                    assert_eq!(simm7.scale_ty, I64);
                    sink.put4(enc_ldst_pair(0b1010100100, simm7, reg, rt, rt2));
                }
                &PairMemArg::PreIndexed(reg, simm7) => {
                    assert_eq!(simm7.scale_ty, I64);
                    sink.put4(enc_ldst_pair(0b1010100110, simm7, reg.to_reg(), rt, rt2));
                }
                &PairMemArg::PostIndexed(reg, simm7) => {
                    assert_eq!(simm7.scale_ty, I64);
                    sink.put4(enc_ldst_pair(0b1010100010, simm7, reg.to_reg(), rt, rt2));
                }
            },
            &Inst::LoadP64 { rt, rt2, ref mem } => {
                let rt = rt.to_reg();
                let rt2 = rt2.to_reg();
                match mem {
                    &PairMemArg::SignedOffset(reg, simm7) => {
                        assert_eq!(simm7.scale_ty, I64);
                        sink.put4(enc_ldst_pair(0b1010100101, simm7, reg, rt, rt2));
                    }
                    &PairMemArg::PreIndexed(reg, simm7) => {
                        assert_eq!(simm7.scale_ty, I64);
                        sink.put4(enc_ldst_pair(0b1010100111, simm7, reg.to_reg(), rt, rt2));
                    }
                    &PairMemArg::PostIndexed(reg, simm7) => {
                        assert_eq!(simm7.scale_ty, I64);
                        sink.put4(enc_ldst_pair(0b1010100011, simm7, reg.to_reg(), rt, rt2));
                    }
                }
            }
            &Inst::Mov { rd, rm } => {
                assert!(rd.to_reg().get_class() == rm.get_class());
                assert!(rm.get_class() == RegClass::I64);
                // MOV to SP is interpreted as MOV to XZR instead. And our codegen
                // should never MOV to XZR.
                assert!(machreg_to_gpr(rd.to_reg()) != 31);
                // Encoded as ORR rd, rm, zero.
                sink.put4(enc_arith_rrr(0b10101010_000, 0b000_000, rd, zero_reg(), rm));
            }
            &Inst::Mov32 { rd, rm } => {
                // MOV to SP is interpreted as MOV to XZR instead. And our codegen
                // should never MOV to XZR.
                assert!(machreg_to_gpr(rd.to_reg()) != 31);
                // Encoded as ORR rd, rm, zero.
                sink.put4(enc_arith_rrr(0b00101010_000, 0b000_000, rd, zero_reg(), rm));
            }
            &Inst::MovZ { rd, imm } => sink.put4(enc_move_wide(MoveWideOpcode::MOVZ, rd, imm)),
            &Inst::MovN { rd, imm } => sink.put4(enc_move_wide(MoveWideOpcode::MOVN, rd, imm)),
            &Inst::MovK { rd, imm } => sink.put4(enc_move_wide(MoveWideOpcode::MOVK, rd, imm)),
            &Inst::CSel { rd, rn, rm, cond } => {
                sink.put4(enc_csel(rd, rn, rm, cond));
            }
            &Inst::CSet { rd, cond } => {
                sink.put4(enc_cset(rd, cond));
            }
            &Inst::FpuMove64 { rd, rn } => {
                sink.put4(enc_vecmov(/* 16b = */ false, rd, rn));
            }
            &Inst::FpuRR { fpu_op, rd, rn } => {
                let top22 = match fpu_op {
                    FPUOp1::Abs32 => 0b000_11110_00_1_000001_10000,
                    FPUOp1::Abs64 => 0b000_11110_01_1_000001_10000,
                    FPUOp1::Neg32 => 0b000_11110_00_1_000010_10000,
                    FPUOp1::Neg64 => 0b000_11110_01_1_000010_10000,
                    FPUOp1::Sqrt32 => 0b000_11110_00_1_000011_10000,
                    FPUOp1::Sqrt64 => 0b000_11110_01_1_000011_10000,
                    FPUOp1::Cvt32To64 => 0b000_11110_00_1_000101_10000,
                    FPUOp1::Cvt64To32 => 0b000_11110_01_1_000100_10000,
                };
                sink.put4(enc_fpurr(top22, rd, rn));
            }
            &Inst::FpuRRR { fpu_op, rd, rn, rm } => {
                let top22 = match fpu_op {
                    FPUOp2::Add32 => 0b000_11110_00_1_00000_001010,
                    FPUOp2::Add64 => 0b000_11110_01_1_00000_001010,
                    FPUOp2::Sub32 => 0b000_11110_00_1_00000_001110,
                    FPUOp2::Sub64 => 0b000_11110_01_1_00000_001110,
                    FPUOp2::Mul32 => 0b000_11110_00_1_00000_000010,
                    FPUOp2::Mul64 => 0b000_11110_01_1_00000_000010,
                    FPUOp2::Div32 => 0b000_11110_00_1_00000_000110,
                    FPUOp2::Div64 => 0b000_11110_01_1_00000_000110,
                    FPUOp2::Max32 => 0b000_11110_00_1_00000_010010,
                    FPUOp2::Max64 => 0b000_11110_01_1_00000_010010,
                    FPUOp2::Min32 => 0b000_11110_00_1_00000_010110,
                    FPUOp2::Min64 => 0b000_11110_01_1_00000_010110,
                };
                sink.put4(enc_fpurrr(top22, rd, rn, rm));
            }
            &Inst::FpuRRRR {
                fpu_op,
                rd,
                rn,
                rm,
                ra,
            } => {
                let top17 = match fpu_op {
                    FPUOp3::MAdd32 => 0b000_11111_00_0_00000_0,
                    FPUOp3::MAdd64 => 0b000_11111_01_0_00000_0,
                };
                sink.put4(enc_fpurrrr(top17, rd, rn, rm, ra));
            }
            &Inst::FpuCmp32 { rn, rm } => {
                sink.put4(enc_fcmp(InstSize::Size32, rn, rm));
            }
            &Inst::FpuCmp64 { rn, rm } => {
                sink.put4(enc_fcmp(InstSize::Size64, rn, rm));
            }
            &Inst::FpuToInt { op, rd, rn } => {
                let top16 = match op {
                    // FCVTZS (32/32-bit)
                    FpuToIntOp::F32ToI32 => 0b000_11110_00_1_11_000,
                    // FCVTZU (32/32-bit)
                    FpuToIntOp::F32ToU32 => 0b000_11110_00_1_11_001,
                    // FCVTZS (32/64-bit)
                    FpuToIntOp::F32ToI64 => 0b100_11110_00_1_11_000,
                    // FCVTZU (32/64-bit)
                    FpuToIntOp::F32ToU64 => 0b100_11110_00_1_11_001,
                    // FCVTZS (64/32-bit)
                    FpuToIntOp::F64ToI32 => 0b000_11110_01_1_11_000,
                    // FCVTZU (64/32-bit)
                    FpuToIntOp::F64ToU32 => 0b000_11110_01_1_11_001,
                    // FCVTZS (64/64-bit)
                    FpuToIntOp::F64ToI64 => 0b100_11110_01_1_11_000,
                    // FCVTZU (64/64-bit)
                    FpuToIntOp::F64ToU64 => 0b100_11110_01_1_11_001,
                };
                sink.put4(enc_fputoint(top16, rd, rn));
            }
            &Inst::IntToFpu { op, rd, rn } => {
                let top16 = match op {
                    // SCVTF (32/32-bit)
                    IntToFpuOp::I32ToF32 => 0b000_11110_00_1_00_010,
                    // UCVTF (32/32-bit)
                    IntToFpuOp::U32ToF32 => 0b000_11110_00_1_00_011,
                    // SCVTF (64/32-bit)
                    IntToFpuOp::I64ToF32 => 0b100_11110_00_1_00_010,
                    // UCVTF (64/32-bit)
                    IntToFpuOp::U64ToF32 => 0b100_11110_00_1_00_011,
                    // SCVTF (32/64-bit)
                    IntToFpuOp::I32ToF64 => 0b000_11110_01_1_00_010,
                    // UCVTF (32/64-bit)
                    IntToFpuOp::U32ToF64 => 0b000_11110_01_1_00_011,
                    // SCVTF (64/64-bit)
                    IntToFpuOp::I64ToF64 => 0b100_11110_01_1_00_010,
                    // UCVTF (64/64-bit)
                    IntToFpuOp::U64ToF64 => 0b100_11110_01_1_00_011,
                };
                sink.put4(enc_inttofpu(top16, rd, rn));
            }
            &Inst::LoadFpuConst32 { rd, const_data } => {
                let inst = Inst::FpuLoad32 {
                    rd,
                    mem: MemArg::Label(MemLabel::PCRel(8)),
                    srcloc: None,
                };
                inst.emit(sink);
                let inst = Inst::Jump {
                    dest: BranchTarget::ResolvedOffset(8),
                };
                inst.emit(sink);
                sink.put4(const_data.to_bits());
            }
            &Inst::LoadFpuConst64 { rd, const_data } => {
                let inst = Inst::FpuLoad64 {
                    rd,
                    mem: MemArg::Label(MemLabel::PCRel(8)),
                    srcloc: None,
                };
                inst.emit(sink);
                let inst = Inst::Jump {
                    dest: BranchTarget::ResolvedOffset(12),
                };
                inst.emit(sink);
                sink.put8(const_data.to_bits());
            }
            &Inst::FpuCSel32 { rd, rn, rm, cond } => {
                sink.put4(enc_fcsel(rd, rn, rm, cond, InstSize::Size32));
            }
            &Inst::FpuCSel64 { rd, rn, rm, cond } => {
                sink.put4(enc_fcsel(rd, rn, rm, cond, InstSize::Size64));
            }
            &Inst::FpuRound { op, rd, rn } => {
                let top22 = match op {
                    FpuRoundMode::Minus32 => 0b000_11110_00_1_001_010_10000,
                    FpuRoundMode::Minus64 => 0b000_11110_01_1_001_010_10000,
                    FpuRoundMode::Plus32 => 0b000_11110_00_1_001_001_10000,
                    FpuRoundMode::Plus64 => 0b000_11110_01_1_001_001_10000,
                    FpuRoundMode::Zero32 => 0b000_11110_00_1_001_011_10000,
                    FpuRoundMode::Zero64 => 0b000_11110_01_1_001_011_10000,
                    FpuRoundMode::Nearest32 => 0b000_11110_00_1_001_000_10000,
                    FpuRoundMode::Nearest64 => 0b000_11110_01_1_001_000_10000,
                };
                sink.put4(enc_fround(top22, rd, rn));
            }
            &Inst::MovToVec64 { rd, rn } => {
                sink.put4(
                    0b010_01110000_01000_0_0011_1_00000_00000
                        | (machreg_to_gpr(rn) << 5)
                        | machreg_to_vec(rd.to_reg()),
                );
            }
            &Inst::MovFromVec64 { rd, rn } => {
                sink.put4(
                    0b010_01110000_01000_0_0111_1_00000_00000
                        | (machreg_to_vec(rn) << 5)
                        | machreg_to_gpr(rd.to_reg()),
                );
            }
            &Inst::VecRRR { rd, rn, rm, alu_op } => {
                let (top11, bit15_10) = match alu_op {
                    VecALUOp::SQAddScalar => (0b010_11110_11_1, 0b000011),
                    VecALUOp::SQSubScalar => (0b010_11110_11_1, 0b001011),
                    VecALUOp::UQAddScalar => (0b011_11110_11_1, 0b000011),
                    VecALUOp::UQSubScalar => (0b011_11110_11_1, 0b001011),
                };
                sink.put4(enc_vec_rrr(top11, rm, bit15_10, rn, rd));
            }
            &Inst::MovToNZCV { rn } => {
                sink.put4(0xd51b4200 | machreg_to_gpr(rn));
            }
            &Inst::MovFromNZCV { rd } => {
                sink.put4(0xd53b4200 | machreg_to_gpr(rd.to_reg()));
            }
            &Inst::CondSet { rd, cond } => {
                sink.put4(
                    0b100_11010100_11111_0000_01_11111_00000
                        | (cond.invert().bits() << 12)
                        | machreg_to_gpr(rd.to_reg()),
                );
            }
            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } if from_bits >= 8 => {
                let top22 = match (signed, from_bits, to_bits) {
                    (false, 8, 32) => 0b010_100110_0_000000_000111, // UXTB (32)
                    (false, 16, 32) => 0b010_100110_0_000000_001111, // UXTH (32)
                    (true, 8, 32) => 0b000_100110_0_000000_000111,  // SXTB (32)
                    (true, 16, 32) => 0b000_100110_0_000000_001111, // SXTH (32)
                    // The 64-bit unsigned variants are the same as the 32-bit ones,
                    // because writes to Wn zero out the top 32 bits of Xn
                    (false, 8, 64) => 0b010_100110_0_000000_000111, // UXTB (64)
                    (false, 16, 64) => 0b010_100110_0_000000_001111, // UXTH (64)
                    (true, 8, 64) => 0b100_100110_1_000000_000111,  // SXTB (64)
                    (true, 16, 64) => 0b100_100110_1_000000_001111, // SXTH (64)
                    // 32-to-64: the unsigned case is a 'mov' (special-cased below).
                    (false, 32, 64) => 0,                           // MOV
                    (true, 32, 64) => 0b100_100110_1_000000_011111, // SXTW (64)
                    _ => panic!(
                        "Unsupported extend combination: signed = {}, from_bits = {}, to_bits = {}",
                        signed, from_bits, to_bits
                    ),
                };
                if top22 != 0 {
                    sink.put4(enc_extend(top22, rd, rn));
                } else {
                    Inst::mov32(rd, rn).emit(sink);
                }
            }
            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } if from_bits == 1 && signed => {
                assert!(to_bits <= 64);
                // Reduce sign-extend-from-1-bit to:
                // - and rd, rn, #1
                // - sub rd, zr, rd

                // We don't have ImmLogic yet, so we just hardcode this. FIXME.
                sink.put4(0x92400000 | (machreg_to_gpr(rn) << 5) | machreg_to_gpr(rd.to_reg()));
                let sub_inst = Inst::AluRRR {
                    alu_op: ALUOp::Sub64,
                    rd,
                    rn: zero_reg(),
                    rm: rd.to_reg(),
                };
                sub_inst.emit(sink);
            }
            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } if from_bits == 1 && !signed => {
                assert!(to_bits <= 64);
                // Reduce zero-extend-from-1-bit to:
                // - and rd, rn, #1

                // We don't have ImmLogic yet, so we just hardcode this. FIXME.
                sink.put4(0x92400000 | (machreg_to_gpr(rn) << 5) | machreg_to_gpr(rd.to_reg()));
            }
            &Inst::Extend { .. } => {
                panic!("Unsupported extend variant");
            }
            &Inst::Jump { ref dest } => {
                // TODO: differentiate between as_off26() returning `None` for
                // out-of-range vs. not-yet-finalized. The latter happens when we
                // do early (fake) emission for size computation.
                sink.put4(enc_jump26(0b000101, dest.as_off26().unwrap()));
            }
            &Inst::Ret => {
                sink.put4(0xd65f03c0);
            }
            &Inst::EpiloguePlaceholder => {
                // Noop; this is just a placeholder for epilogues.
            }
            &Inst::Call {
                ref dest,
                loc,
                opcode,
                ..
            } => {
                sink.add_reloc(loc, Reloc::Arm64Call, dest, 0);
                sink.put4(enc_jump26(0b100101, 0));
                if opcode.is_call() {
                    sink.add_call_site(loc, opcode);
                }
            }
            &Inst::CallInd {
                rn, loc, opcode, ..
            } => {
                sink.put4(0b1101011_0001_11111_000000_00000_00000 | (machreg_to_gpr(rn) << 5));
                if opcode.is_call() {
                    sink.add_call_site(loc, opcode);
                }
            }
            &Inst::CondBr { .. } => panic!("Unlowered CondBr during binemit!"),
            &Inst::CondBrLowered { target, kind } => match kind {
                // TODO: handle >2^19 case by emitting a compound sequence with
                // an unconditional (26-bit) branch. We need branch-relaxation
                // adjustment machinery to enable this (because we don't want to
                // always emit the long form).
                CondBrKind::Zero(reg) => {
                    sink.put4(enc_cmpbr(0b1_011010_0, target.as_off19().unwrap(), reg));
                }
                CondBrKind::NotZero(reg) => {
                    sink.put4(enc_cmpbr(0b1_011010_1, target.as_off19().unwrap(), reg));
                }
                CondBrKind::Cond(c) => {
                    sink.put4(enc_cbr(
                        0b01010100,
                        target.as_off19().unwrap_or(0),
                        0b0,
                        c.bits(),
                    ));
                }
            },
            &Inst::CondBrLoweredCompound {
                taken,
                not_taken,
                kind,
            } => {
                // Conditional part first.
                match kind {
                    CondBrKind::Zero(reg) => {
                        sink.put4(enc_cmpbr(0b1_011010_0, taken.as_off19().unwrap(), reg));
                    }
                    CondBrKind::NotZero(reg) => {
                        sink.put4(enc_cmpbr(0b1_011010_1, taken.as_off19().unwrap(), reg));
                    }
                    CondBrKind::Cond(c) => {
                        sink.put4(enc_cbr(
                            0b01010100,
                            taken.as_off19().unwrap_or(0),
                            0b0,
                            c.bits(),
                        ));
                    }
                }
                // Unconditional part.
                sink.put4(enc_jump26(0b000101, not_taken.as_off26().unwrap_or(0)));
            }
            &Inst::IndirectBr { rn, .. } => {
                sink.put4(enc_br(rn));
            }
            &Inst::Nop0 => {}
            &Inst::Nop4 => {
                sink.put4(0xd503201f);
            }
            &Inst::Brk => {
                sink.put4(0xd4200000);
            }
            &Inst::Udf { trap_info } => {
                let (srcloc, code) = trap_info;
                sink.add_trap(srcloc, code);
                sink.put4(0xd4a00000);
            }
            &Inst::Adr { rd, ref label } => {
                let off = memlabel_finalize(sink.cur_offset_from_start(), label);
                assert!(off > -(1 << 20));
                assert!(off < (1 << 20));
                sink.put4(enc_adr(off, rd));
            }
            &Inst::Word4 { data } => {
                sink.put4(data);
            }
            &Inst::Word8 { data } => {
                sink.put8(data);
            }
            &Inst::JTSequence {
                ridx,
                rtmp1,
                rtmp2,
                ref targets,
                ..
            } => {
                // This sequence is *one* instruction in the vcode, and is expanded only here at
                // emission time, because we cannot allow the regalloc to insert spills/reloads in
                // the middle; we depend on hardcoded PC-rel addressing below.
                //
                // N.B.: if PC-rel addressing on ADR below is changed, also update
                // `Inst::with_block_offsets()` in aarch64/inst/mod.rs.

                // Save index in a tmp (the live range of ridx only goes to start of this
                // sequence; rtmp1 or rtmp2 may overwrite it).
                let inst = Inst::gen_move(rtmp2, ridx, I64);
                inst.emit(sink);
                // Load address of jump table
                let inst = Inst::Adr {
                    rd: rtmp1,
                    label: MemLabel::PCRel(16),
                };
                inst.emit(sink);
                // Load value out of jump table
                let inst = Inst::SLoad32 {
                    rd: rtmp2,
                    mem: MemArg::reg_plus_reg_scaled_extended(
                        rtmp1.to_reg(),
                        rtmp2.to_reg(),
                        I32,
                        ExtendOp::UXTW,
                    ),
                    srcloc: None, // can't cause a user trap.
                };
                inst.emit(sink);
                // Add base of jump table to jump-table-sourced block offset
                let inst = Inst::AluRRR {
                    alu_op: ALUOp::Add64,
                    rd: rtmp1,
                    rn: rtmp1.to_reg(),
                    rm: rtmp2.to_reg(),
                };
                inst.emit(sink);
                // Branch to computed address. (`targets` here is only used for successor queries
                // and is not needed for emission.)
                let inst = Inst::IndirectBr {
                    rn: rtmp1.to_reg(),
                    targets: vec![],
                };
                inst.emit(sink);
                // Emit jump table (table of 32-bit offsets).
                for target in targets {
                    let off = target.as_offset_words() * 4;
                    let off = i32::try_from(off).unwrap();
                    // cast i32 to u32 (two's-complement)
                    let off = off as u32;
                    sink.put4(off);
                }
            }
            &Inst::LoadConst64 { rd, const_data } => {
                let inst = Inst::ULoad64 {
                    rd,
                    mem: MemArg::Label(MemLabel::PCRel(8)),
                    srcloc: None, // can't cause a user trap.
                };
                inst.emit(sink);
                let inst = Inst::Jump {
                    dest: BranchTarget::ResolvedOffset(12),
                };
                inst.emit(sink);
                sink.put8(const_data);
            }
            &Inst::LoadExtName {
                rd,
                ref name,
                offset,
                srcloc,
            } => {
                let inst = Inst::ULoad64 {
                    rd,
                    mem: MemArg::Label(MemLabel::PCRel(8)),
                    srcloc: None, // can't cause a user trap.
                };
                inst.emit(sink);
                let inst = Inst::Jump {
                    dest: BranchTarget::ResolvedOffset(12),
                };
                inst.emit(sink);
                sink.add_reloc(srcloc, Reloc::Abs8, name, offset);
                sink.put8(0);
            }
            &Inst::LoadAddr { rd, ref mem } => match *mem {
                MemArg::FPOffset(fp_off) => {
                    let alu_op = if fp_off < 0 {
                        ALUOp::Sub64
                    } else {
                        ALUOp::Add64
                    };
                    if let Some(imm12) = Imm12::maybe_from_u64(u64::try_from(fp_off.abs()).unwrap())
                    {
                        let inst = Inst::AluRRImm12 {
                            alu_op,
                            rd,
                            imm12,
                            rn: fp_reg(),
                        };
                        inst.emit(sink);
                    } else {
                        let const_insts =
                            Inst::load_constant(rd, u64::try_from(fp_off.abs()).unwrap());
                        for inst in const_insts {
                            inst.emit(sink);
                        }
                        let inst = Inst::AluRRR {
                            alu_op,
                            rd,
                            rn: fp_reg(),
                            rm: rd.to_reg(),
                        };
                        inst.emit(sink);
                    }
                }
                _ => unimplemented!("{:?}", mem),
            },
            &Inst::GetPinnedReg { rd } => {
                let inst = Inst::Mov {
                    rd,
                    rm: xreg(PINNED_REG),
                };
                inst.emit(sink);
            }
            &Inst::SetPinnedReg { rm } => {
                let inst = Inst::Mov {
                    rd: Writable::from_reg(xreg(PINNED_REG)),
                    rm,
                };
                inst.emit(sink);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::isa::test_utils;
    use crate::settings;

    #[test]
    fn test_aarch64_binemit() {
        let mut insns = Vec::<(Inst, &str, &str)>::new();

        // N.B.: the architecture is little-endian, so when transcribing the 32-bit
        // hex instructions from e.g. objdump disassembly, one must swap the bytes
        // seen below. (E.g., a `ret` is normally written as the u32 `D65F03C0`,
        // but we write it here as C0035FD6.)

        // Useful helper script to produce the encodings from the text:
        //
        //      #!/bin/sh
        //      tmp=`mktemp /tmp/XXXXXXXX.o`
        //      aarch64-linux-gnu-as /dev/stdin -o $tmp
        //      aarch64-linux-gnu-objdump -d $tmp
        //      rm -f $tmp
        //
        // Then:
        //
        //      $ echo "mov x1, x2" | aarch64inst.sh
        insns.push((Inst::Ret, "C0035FD6", "ret"));
        insns.push((Inst::Nop0, "", "nop-zero-len"));
        insns.push((Inst::Nop4, "1F2003D5", "nop"));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Add32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
            },
            "4100030B",
            "add w1, w2, w3",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Add64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400068B",
            "add x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Sub32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
            },
            "4100034B",
            "sub w1, w2, w3",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Sub64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40006CB",
            "sub x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Orr32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
            },
            "4100032A",
            "orr w1, w2, w3",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Orr64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40006AA",
            "orr x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::And32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
            },
            "4100030A",
            "and w1, w2, w3",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::And64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400068A",
            "and x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::SubS32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
            },
            "4100036B",
            "subs w1, w2, w3",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::SubS64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40006EB",
            "subs x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::AddS32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
            },
            "4100032B",
            "adds w1, w2, w3",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::AddS64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40006AB",
            "adds x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::SDiv64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40CC69A",
            "sdiv x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::UDiv64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A408C69A",
            "udiv x4, x5, x6",
        ));

        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Eor32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400064A",
            "eor w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Eor64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40006CA",
            "eor x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::AndNot32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400260A",
            "bic w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::AndNot64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400268A",
            "bic x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::OrrNot32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400262A",
            "orn w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::OrrNot64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40026AA",
            "orn x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::EorNot32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A400264A",
            "eon w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::EorNot64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A40026CA",
            "eon x4, x5, x6",
        ));

        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::RotR32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A42CC61A",
            "ror w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::RotR64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A42CC69A",
            "ror x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Lsr32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A424C61A",
            "lsr w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Lsr64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A424C69A",
            "lsr x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Asr32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A428C61A",
            "asr w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Asr64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A428C69A",
            "asr x4, x5, x6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Lsl32,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A420C61A",
            "lsl w4, w5, w6",
        ));
        insns.push((
            Inst::AluRRR {
                alu_op: ALUOp::Lsl64,
                rd: writable_xreg(4),
                rn: xreg(5),
                rm: xreg(6),
            },
            "A420C69A",
            "lsl x4, x5, x6",
        ));

        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::Add32,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: false,
                },
            },
            "078D0411",
            "add w7, w8, #291",
        ));
        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::Add32,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: true,
                },
            },
            "078D4411",
            "add w7, w8, #1191936",
        ));
        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: false,
                },
            },
            "078D0491",
            "add x7, x8, #291",
        ));
        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::Sub32,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: false,
                },
            },
            "078D0451",
            "sub w7, w8, #291",
        ));
        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::Sub64,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: false,
                },
            },
            "078D04D1",
            "sub x7, x8, #291",
        ));
        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::SubS32,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: false,
                },
            },
            "078D0471",
            "subs w7, w8, #291",
        ));
        insns.push((
            Inst::AluRRImm12 {
                alu_op: ALUOp::SubS64,
                rd: writable_xreg(7),
                rn: xreg(8),
                imm12: Imm12 {
                    bits: 0x123,
                    shift12: false,
                },
            },
            "078D04F1",
            "subs x7, x8, #291",
        ));

        insns.push((
            Inst::AluRRRExtend {
                alu_op: ALUOp::Add32,
                rd: writable_xreg(7),
                rn: xreg(8),
                rm: xreg(9),
                extendop: ExtendOp::SXTB,
            },
            "0781290B",
            "add w7, w8, w9, SXTB",
        ));

        insns.push((
            Inst::AluRRRExtend {
                alu_op: ALUOp::Add64,
                rd: writable_xreg(15),
                rn: xreg(16),
                rm: xreg(17),
                extendop: ExtendOp::UXTB,
            },
            "0F02318B",
            "add x15, x16, x17, UXTB",
        ));

        insns.push((
            Inst::AluRRRExtend {
                alu_op: ALUOp::Sub32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                extendop: ExtendOp::SXTH,
            },
            "41A0234B",
            "sub w1, w2, w3, SXTH",
        ));

        insns.push((
            Inst::AluRRRExtend {
                alu_op: ALUOp::Sub64,
                rd: writable_xreg(20),
                rn: xreg(21),
                rm: xreg(22),
                extendop: ExtendOp::UXTW,
            },
            "B44236CB",
            "sub x20, x21, x22, UXTW",
        ));

        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Add32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(20).unwrap(),
                ),
            },
            "6A510C0B",
            "add w10, w11, w12, LSL 20",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Add64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::ASR,
                    ShiftOpShiftImm::maybe_from_shift(42).unwrap(),
                ),
            },
            "6AA98C8B",
            "add x10, x11, x12, ASR 42",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Sub32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C4B",
            "sub w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Sub64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0CCB",
            "sub x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Orr32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C2A",
            "orr w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Orr64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0CAA",
            "orr x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::And32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C0A",
            "and w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::And64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C8A",
            "and x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Eor32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C4A",
            "eor w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::Eor64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0CCA",
            "eor x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::OrrNot32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D2C2A",
            "orn w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::OrrNot64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D2CAA",
            "orn x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::AndNot32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D2C0A",
            "bic w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::AndNot64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D2C8A",
            "bic x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::EorNot32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D2C4A",
            "eon w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::EorNot64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D2CCA",
            "eon x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::AddS32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C2B",
            "adds w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::AddS64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0CAB",
            "adds x10, x11, x12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::SubS32,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0C6B",
            "subs w10, w11, w12, LSL 23",
        ));
        insns.push((
            Inst::AluRRRShift {
                alu_op: ALUOp::SubS64,
                rd: writable_xreg(10),
                rn: xreg(11),
                rm: xreg(12),
                shiftop: ShiftOpAndAmt::new(
                    ShiftOp::LSL,
                    ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
                ),
            },
            "6A5D0CEB",
            "subs x10, x11, x12, LSL 23",
        ));

        insns.push((
            Inst::AluRRRR {
                alu_op: ALUOp::MAdd32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                ra: xreg(4),
            },
            "4110031B",
            "madd w1, w2, w3, w4",
        ));
        insns.push((
            Inst::AluRRRR {
                alu_op: ALUOp::MAdd64,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                ra: xreg(4),
            },
            "4110039B",
            "madd x1, x2, x3, x4",
        ));
        insns.push((
            Inst::AluRRRR {
                alu_op: ALUOp::MSub32,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                ra: xreg(4),
            },
            "4190031B",
            "msub w1, w2, w3, w4",
        ));
        insns.push((
            Inst::AluRRRR {
                alu_op: ALUOp::MSub64,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                ra: xreg(4),
            },
            "4190039B",
            "msub x1, x2, x3, x4",
        ));
        insns.push((
            Inst::AluRRRR {
                alu_op: ALUOp::SMulH,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                ra: zero_reg(),
            },
            "417C439B",
            "smulh x1, x2, x3",
        ));
        insns.push((
            Inst::AluRRRR {
                alu_op: ALUOp::UMulH,
                rd: writable_xreg(1),
                rn: xreg(2),
                rm: xreg(3),
                ra: zero_reg(),
            },
            "417CC39B",
            "umulh x1, x2, x3",
        ));

        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::RotR32,
                rd: writable_xreg(20),
                rn: xreg(21),
                immshift: ImmShift::maybe_from_u64(19).unwrap(),
            },
            "B44E9513",
            "ror w20, w21, #19",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::RotR64,
                rd: writable_xreg(20),
                rn: xreg(21),
                immshift: ImmShift::maybe_from_u64(42).unwrap(),
            },
            "B4AAD593",
            "ror x20, x21, #42",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::Lsr32,
                rd: writable_xreg(10),
                rn: xreg(11),
                immshift: ImmShift::maybe_from_u64(13).unwrap(),
            },
            "6A7D0D53",
            "lsr w10, w11, #13",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::Lsr64,
                rd: writable_xreg(10),
                rn: xreg(11),
                immshift: ImmShift::maybe_from_u64(57).unwrap(),
            },
            "6AFD79D3",
            "lsr x10, x11, #57",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::Asr32,
                rd: writable_xreg(4),
                rn: xreg(5),
                immshift: ImmShift::maybe_from_u64(7).unwrap(),
            },
            "A47C0713",
            "asr w4, w5, #7",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::Asr64,
                rd: writable_xreg(4),
                rn: xreg(5),
                immshift: ImmShift::maybe_from_u64(35).unwrap(),
            },
            "A4FC6393",
            "asr x4, x5, #35",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::Lsl32,
                rd: writable_xreg(8),
                rn: xreg(9),
                immshift: ImmShift::maybe_from_u64(24).unwrap(),
            },
            "281D0853",
            "lsl w8, w9, #24",
        ));
        insns.push((
            Inst::AluRRImmShift {
                alu_op: ALUOp::Lsl64,
                rd: writable_xreg(8),
                rn: xreg(9),
                immshift: ImmShift::maybe_from_u64(63).unwrap(),
            },
            "280141D3",
            "lsl x8, x9, #63",
        ));

        insns.push((
            Inst::AluRRImmLogic {
                alu_op: ALUOp::And32,
                rd: writable_xreg(21),
                rn: xreg(27),
                imml: ImmLogic::maybe_from_u64(0x80003fff, I32).unwrap(),
            },
            "753B0112",
            "and w21, w27, #2147500031",
        ));
        insns.push((
            Inst::AluRRImmLogic {
                alu_op: ALUOp::And64,
                rd: writable_xreg(7),
                rn: xreg(6),
                imml: ImmLogic::maybe_from_u64(0x3fff80003fff800, I64).unwrap(),
            },
            "C7381592",
            "and x7, x6, #288221580125796352",
        ));
        insns.push((
            Inst::AluRRImmLogic {
                alu_op: ALUOp::Orr32,
                rd: writable_xreg(1),
                rn: xreg(5),
                imml: ImmLogic::maybe_from_u64(0x100000, I32).unwrap(),
            },
            "A1000C32",
            "orr w1, w5, #1048576",
        ));
        insns.push((
            Inst::AluRRImmLogic {
                alu_op: ALUOp::Orr64,
                rd: writable_xreg(4),
                rn: xreg(5),
                imml: ImmLogic::maybe_from_u64(0x8181818181818181, I64).unwrap(),
            },
            "A4C401B2",
            "orr x4, x5, #9331882296111890817",
        ));
        insns.push((
            Inst::AluRRImmLogic {
                alu_op: ALUOp::Eor32,
                rd: writable_xreg(1),
                rn: xreg(5),
                imml: ImmLogic::maybe_from_u64(0x00007fff, I32).unwrap(),
            },
            "A1380052",
            "eor w1, w5, #32767",
        ));
        insns.push((
            Inst::AluRRImmLogic {
                alu_op: ALUOp::Eor64,
                rd: writable_xreg(10),
                rn: xreg(8),
                imml: ImmLogic::maybe_from_u64(0x8181818181818181, I64).unwrap(),
            },
            "0AC501D2",
            "eor x10, x8, #9331882296111890817",
        ));

        insns.push((
            Inst::BitRR {
                op: BitOp::RBit32,
                rd: writable_xreg(1),
                rn: xreg(10),
            },
            "4101C05A",
            "rbit w1, w10",
        ));

        insns.push((
            Inst::BitRR {
                op: BitOp::RBit64,
                rd: writable_xreg(1),
                rn: xreg(10),
            },
            "4101C0DA",
            "rbit x1, x10",
        ));

        insns.push((
            Inst::BitRR {
                op: BitOp::Clz32,
                rd: writable_xreg(15),
                rn: xreg(3),
            },
            "6F10C05A",
            "clz w15, w3",
        ));

        insns.push((
            Inst::BitRR {
                op: BitOp::Clz64,
                rd: writable_xreg(15),
                rn: xreg(3),
            },
            "6F10C0DA",
            "clz x15, x3",
        ));

        insns.push((
            Inst::BitRR {
                op: BitOp::Cls32,
                rd: writable_xreg(21),
                rn: xreg(16),
            },
            "1516C05A",
            "cls w21, w16",
        ));

        insns.push((
            Inst::BitRR {
                op: BitOp::Cls64,
                rd: writable_xreg(21),
                rn: xreg(16),
            },
            "1516C0DA",
            "cls x21, x16",
        ));

        insns.push((
            Inst::ULoad8 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "41004038",
            "ldurb w1, [x2]",
        ));
        insns.push((
            Inst::ULoad8 {
                rd: writable_xreg(1),
                mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::zero(I8)),
                srcloc: None,
            },
            "41004039",
            "ldrb w1, [x2]",
        ));
        insns.push((
            Inst::ULoad8 {
                rd: writable_xreg(1),
                mem: MemArg::RegReg(xreg(2), xreg(5)),
                srcloc: None,
            },
            "41686538",
            "ldrb w1, [x2, x5]",
        ));
        insns.push((
            Inst::SLoad8 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "41008038",
            "ldursb x1, [x2]",
        ));
        insns.push((
            Inst::SLoad8 {
                rd: writable_xreg(1),
                mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(63, I8).unwrap()),
                srcloc: None,
            },
            "41FC8039",
            "ldrsb x1, [x2, #63]",
        ));
        insns.push((
            Inst::SLoad8 {
                rd: writable_xreg(1),
                mem: MemArg::RegReg(xreg(2), xreg(5)),
                srcloc: None,
            },
            "4168A538",
            "ldrsb x1, [x2, x5]",
        ));
        insns.push((
            Inst::ULoad16 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::maybe_from_i64(5).unwrap()),
                srcloc: None,
            },
            "41504078",
            "ldurh w1, [x2, #5]",
        ));
        insns.push((
            Inst::ULoad16 {
                rd: writable_xreg(1),
                mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(8, I16).unwrap()),
                srcloc: None,
            },
            "41104079",
            "ldrh w1, [x2, #8]",
        ));
        insns.push((
            Inst::ULoad16 {
                rd: writable_xreg(1),
                mem: MemArg::RegScaled(xreg(2), xreg(3), I16),
                srcloc: None,
            },
            "41786378",
            "ldrh w1, [x2, x3, LSL #1]",
        ));
        insns.push((
            Inst::SLoad16 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "41008078",
            "ldursh x1, [x2]",
        ));
        insns.push((
            Inst::SLoad16 {
                rd: writable_xreg(28),
                mem: MemArg::UnsignedOffset(
                    xreg(20),
                    UImm12Scaled::maybe_from_i64(24, I16).unwrap(),
                ),
                srcloc: None,
            },
            "9C328079",
            "ldrsh x28, [x20, #24]",
        ));
        insns.push((
            Inst::SLoad16 {
                rd: writable_xreg(28),
                mem: MemArg::RegScaled(xreg(20), xreg(20), I16),
                srcloc: None,
            },
            "9C7AB478",
            "ldrsh x28, [x20, x20, LSL #1]",
        ));
        insns.push((
            Inst::ULoad32 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "410040B8",
            "ldur w1, [x2]",
        ));
        insns.push((
            Inst::ULoad32 {
                rd: writable_xreg(12),
                mem: MemArg::UnsignedOffset(
                    xreg(0),
                    UImm12Scaled::maybe_from_i64(204, I32).unwrap(),
                ),
                srcloc: None,
            },
            "0CCC40B9",
            "ldr w12, [x0, #204]",
        ));
        insns.push((
            Inst::ULoad32 {
                rd: writable_xreg(1),
                mem: MemArg::RegScaled(xreg(2), xreg(12), I32),
                srcloc: None,
            },
            "41786CB8",
            "ldr w1, [x2, x12, LSL #2]",
        ));
        insns.push((
            Inst::SLoad32 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "410080B8",
            "ldursw x1, [x2]",
        ));
        insns.push((
            Inst::SLoad32 {
                rd: writable_xreg(12),
                mem: MemArg::UnsignedOffset(
                    xreg(1),
                    UImm12Scaled::maybe_from_i64(16380, I32).unwrap(),
                ),
                srcloc: None,
            },
            "2CFCBFB9",
            "ldrsw x12, [x1, #16380]",
        ));
        insns.push((
            Inst::SLoad32 {
                rd: writable_xreg(1),
                mem: MemArg::RegScaled(xreg(5), xreg(1), I32),
                srcloc: None,
            },
            "A178A1B8",
            "ldrsw x1, [x5, x1, LSL #2]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "410040F8",
            "ldur x1, [x2]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::maybe_from_i64(-256).unwrap()),
                srcloc: None,
            },
            "410050F8",
            "ldur x1, [x2, #-256]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::maybe_from_i64(255).unwrap()),
                srcloc: None,
            },
            "41F04FF8",
            "ldur x1, [x2, #255]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::UnsignedOffset(
                    xreg(2),
                    UImm12Scaled::maybe_from_i64(32760, I64).unwrap(),
                ),
                srcloc: None,
            },
            "41FC7FF9",
            "ldr x1, [x2, #32760]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::RegReg(xreg(2), xreg(3)),
                srcloc: None,
            },
            "416863F8",
            "ldr x1, [x2, x3]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::RegScaled(xreg(2), xreg(3), I64),
                srcloc: None,
            },
            "417863F8",
            "ldr x1, [x2, x3, LSL #3]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::RegScaledExtended(xreg(2), xreg(3), I64, ExtendOp::SXTW),
                srcloc: None,
            },
            "41D863F8",
            "ldr x1, [x2, w3, SXTW #3]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::Label(MemLabel::PCRel(64)),
                srcloc: None,
            },
            "01020058",
            "ldr x1, pc+64",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::PreIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
                srcloc: None,
            },
            "410C41F8",
            "ldr x1, [x2, #16]!",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::PostIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
                srcloc: None,
            },
            "410441F8",
            "ldr x1, [x2], #16",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::FPOffset(32768),
                srcloc: None,
            },
            "0F0090D2EF011D8BE10140F9",
            "movz x15, #32768 ; add x15, x15, fp ; ldr x1, [x15]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::FPOffset(-32768),
                srcloc: None,
            },
            "EFFF8F92EF011D8BE10140F9",
            "movn x15, #32767 ; add x15, x15, fp ; ldr x1, [x15]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::FPOffset(1048576), // 2^20
                srcloc: None,
            },
            "0F02A0D2EF011D8BE10140F9",
            "movz x15, #16, LSL #16 ; add x15, x15, fp ; ldr x1, [x15]",
        ));
        insns.push((
            Inst::ULoad64 {
                rd: writable_xreg(1),
                mem: MemArg::FPOffset(1048576 + 1), // 2^20 + 1
                srcloc: None,
            },
            "2F0080D20F02A0F2EF011D8BE10140F9",
            "movz x15, #1 ; movk x15, #16, LSL #16 ; add x15, x15, fp ; ldr x1, [x15]",
        ));

        insns.push((
            Inst::Store8 {
                rd: xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "41000038",
            "sturb w1, [x2]",
        ));
        insns.push((
            Inst::Store8 {
                rd: xreg(1),
                mem: MemArg::UnsignedOffset(
                    xreg(2),
                    UImm12Scaled::maybe_from_i64(4095, I8).unwrap(),
                ),
                srcloc: None,
            },
            "41FC3F39",
            "strb w1, [x2, #4095]",
        ));
        insns.push((
            Inst::Store16 {
                rd: xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "41000078",
            "sturh w1, [x2]",
        ));
        insns.push((
            Inst::Store16 {
                rd: xreg(1),
                mem: MemArg::UnsignedOffset(
                    xreg(2),
                    UImm12Scaled::maybe_from_i64(8190, I16).unwrap(),
                ),
                srcloc: None,
            },
            "41FC3F79",
            "strh w1, [x2, #8190]",
        ));
        insns.push((
            Inst::Store32 {
                rd: xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "410000B8",
            "stur w1, [x2]",
        ));
        insns.push((
            Inst::Store32 {
                rd: xreg(1),
                mem: MemArg::UnsignedOffset(
                    xreg(2),
                    UImm12Scaled::maybe_from_i64(16380, I32).unwrap(),
                ),
                srcloc: None,
            },
            "41FC3FB9",
            "str w1, [x2, #16380]",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
                srcloc: None,
            },
            "410000F8",
            "stur x1, [x2]",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::UnsignedOffset(
                    xreg(2),
                    UImm12Scaled::maybe_from_i64(32760, I64).unwrap(),
                ),
                srcloc: None,
            },
            "41FC3FF9",
            "str x1, [x2, #32760]",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::RegReg(xreg(2), xreg(3)),
                srcloc: None,
            },
            "416823F8",
            "str x1, [x2, x3]",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::RegScaled(xreg(2), xreg(3), I64),
                srcloc: None,
            },
            "417823F8",
            "str x1, [x2, x3, LSL #3]",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::RegScaledExtended(xreg(2), xreg(3), I64, ExtendOp::UXTW),
                srcloc: None,
            },
            "415823F8",
            "str x1, [x2, w3, UXTW #3]",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::PreIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
                srcloc: None,
            },
            "410C01F8",
            "str x1, [x2, #16]!",
        ));
        insns.push((
            Inst::Store64 {
                rd: xreg(1),
                mem: MemArg::PostIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
                srcloc: None,
            },
            "410401F8",
            "str x1, [x2], #16",
        ));

        insns.push((
            Inst::StoreP64 {
                rt: xreg(8),
                rt2: xreg(9),
                mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::zero(I64)),
            },
            "482500A9",
            "stp x8, x9, [x10]",
        ));
        insns.push((
            Inst::StoreP64 {
                rt: xreg(8),
                rt2: xreg(9),
                mem: PairMemArg::SignedOffset(
                    xreg(10),
                    SImm7Scaled::maybe_from_i64(504, I64).unwrap(),
                ),
            },
            "48A51FA9",
            "stp x8, x9, [x10, #504]",
        ));
        insns.push((
            Inst::StoreP64 {
                rt: xreg(8),
                rt2: xreg(9),
                mem: PairMemArg::SignedOffset(
                    xreg(10),
                    SImm7Scaled::maybe_from_i64(-64, I64).unwrap(),
                ),
            },
            "48253CA9",
            "stp x8, x9, [x10, #-64]",
        ));
        insns.push((
            Inst::StoreP64 {
                rt: xreg(21),
                rt2: xreg(28),
                mem: PairMemArg::SignedOffset(
                    xreg(1),
                    SImm7Scaled::maybe_from_i64(-512, I64).unwrap(),
                ),
            },
            "357020A9",
            "stp x21, x28, [x1, #-512]",
        ));
        insns.push((
            Inst::StoreP64 {
                rt: xreg(8),
                rt2: xreg(9),
                mem: PairMemArg::PreIndexed(
                    writable_xreg(10),
                    SImm7Scaled::maybe_from_i64(-64, I64).unwrap(),
                ),
            },
            "4825BCA9",
            "stp x8, x9, [x10, #-64]!",
        ));
        insns.push((
            Inst::StoreP64 {
                rt: xreg(15),
                rt2: xreg(16),
                mem: PairMemArg::PostIndexed(
                    writable_xreg(20),
                    SImm7Scaled::maybe_from_i64(504, I64).unwrap(),
                ),
            },
            "8FC29FA8",
            "stp x15, x16, [x20], #504",
        ));

        insns.push((
            Inst::LoadP64 {
                rt: writable_xreg(8),
                rt2: writable_xreg(9),
                mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::zero(I64)),
            },
            "482540A9",
            "ldp x8, x9, [x10]",
        ));
        insns.push((
            Inst::LoadP64 {
                rt: writable_xreg(8),
                rt2: writable_xreg(9),
                mem: PairMemArg::SignedOffset(
                    xreg(10),
                    SImm7Scaled::maybe_from_i64(504, I64).unwrap(),
                ),
            },
            "48A55FA9",
            "ldp x8, x9, [x10, #504]",
        ));
        insns.push((
            Inst::LoadP64 {
                rt: writable_xreg(8),
                rt2: writable_xreg(9),
                mem: PairMemArg::SignedOffset(
                    xreg(10),
                    SImm7Scaled::maybe_from_i64(-64, I64).unwrap(),
                ),
            },
            "48257CA9",
            "ldp x8, x9, [x10, #-64]",
        ));
        insns.push((
            Inst::LoadP64 {
                rt: writable_xreg(8),
                rt2: writable_xreg(9),
                mem: PairMemArg::SignedOffset(
                    xreg(10),
                    SImm7Scaled::maybe_from_i64(-512, I64).unwrap(),
                ),
            },
            "482560A9",
            "ldp x8, x9, [x10, #-512]",
        ));
        insns.push((
            Inst::LoadP64 {
                rt: writable_xreg(8),
                rt2: writable_xreg(9),
                mem: PairMemArg::PreIndexed(
                    writable_xreg(10),
                    SImm7Scaled::maybe_from_i64(-64, I64).unwrap(),
                ),
            },
            "4825FCA9",
            "ldp x8, x9, [x10, #-64]!",
        ));
        insns.push((
            Inst::LoadP64 {
                rt: writable_xreg(8),
                rt2: writable_xreg(25),
                mem: PairMemArg::PostIndexed(
                    writable_xreg(12),
                    SImm7Scaled::maybe_from_i64(504, I64).unwrap(),
                ),
            },
            "88E5DFA8",
            "ldp x8, x25, [x12], #504",
        ));

        insns.push((
            Inst::Mov {
                rd: writable_xreg(8),
                rm: xreg(9),
            },
            "E80309AA",
            "mov x8, x9",
        ));
        insns.push((
            Inst::Mov32 {
                rd: writable_xreg(8),
                rm: xreg(9),
            },
            "E803092A",
            "mov w8, w9",
        ));

        insns.push((
            Inst::MovZ {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_ffff).unwrap(),
            },
            "E8FF9FD2",
            "movz x8, #65535",
        ));
        insns.push((
            Inst::MovZ {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_ffff_0000).unwrap(),
            },
            "E8FFBFD2",
            "movz x8, #65535, LSL #16",
        ));
        insns.push((
            Inst::MovZ {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_ffff_0000_0000).unwrap(),
            },
            "E8FFDFD2",
            "movz x8, #65535, LSL #32",
        ));
        insns.push((
            Inst::MovZ {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0xffff_0000_0000_0000).unwrap(),
            },
            "E8FFFFD2",
            "movz x8, #65535, LSL #48",
        ));

        insns.push((
            Inst::MovN {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_ffff).unwrap(),
            },
            "E8FF9F92",
            "movn x8, #65535",
        ));
        insns.push((
            Inst::MovN {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_ffff_0000).unwrap(),
            },
            "E8FFBF92",
            "movn x8, #65535, LSL #16",
        ));
        insns.push((
            Inst::MovN {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_ffff_0000_0000).unwrap(),
            },
            "E8FFDF92",
            "movn x8, #65535, LSL #32",
        ));
        insns.push((
            Inst::MovN {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0xffff_0000_0000_0000).unwrap(),
            },
            "E8FFFF92",
            "movn x8, #65535, LSL #48",
        ));

        insns.push((
            Inst::MovK {
                rd: writable_xreg(12),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_0000).unwrap(),
            },
            "0C0080F2",
            "movk x12, #0",
        ));
        insns.push((
            Inst::MovK {
                rd: writable_xreg(19),
                imm: MoveWideConst::maybe_with_shift(0x0000, 16).unwrap(),
            },
            "1300A0F2",
            "movk x19, #0, LSL #16",
        ));
        insns.push((
            Inst::MovK {
                rd: writable_xreg(3),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_ffff).unwrap(),
            },
            "E3FF9FF2",
            "movk x3, #65535",
        ));
        insns.push((
            Inst::MovK {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_0000_ffff_0000).unwrap(),
            },
            "E8FFBFF2",
            "movk x8, #65535, LSL #16",
        ));
        insns.push((
            Inst::MovK {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0x0000_ffff_0000_0000).unwrap(),
            },
            "E8FFDFF2",
            "movk x8, #65535, LSL #32",
        ));
        insns.push((
            Inst::MovK {
                rd: writable_xreg(8),
                imm: MoveWideConst::maybe_from_u64(0xffff_0000_0000_0000).unwrap(),
            },
            "E8FFFFF2",
            "movk x8, #65535, LSL #48",
        ));

        insns.push((
            Inst::CSel {
                rd: writable_xreg(10),
                rn: xreg(12),
                rm: xreg(14),
                cond: Cond::Hs,
            },
            "8A218E9A",
            "csel x10, x12, x14, hs",
        ));
        insns.push((
            Inst::CSet {
                rd: writable_xreg(15),
                cond: Cond::Ge,
            },
            "EFB79F9A",
            "cset x15, ge",
        ));
        insns.push((
            Inst::MovToVec64 {
                rd: writable_vreg(20),
                rn: xreg(21),
            },
            "B41E084E",
            "mov v20.d[0], x21",
        ));
        insns.push((
            Inst::MovFromVec64 {
                rd: writable_xreg(21),
                rn: vreg(20),
            },
            "953E084E",
            "mov x21, v20.d[0]",
        ));
        insns.push((
            Inst::MovToNZCV { rn: xreg(13) },
            "0D421BD5",
            "msr nzcv, x13",
        ));
        insns.push((
            Inst::MovFromNZCV {
                rd: writable_xreg(27),
            },
            "1B423BD5",
            "mrs x27, nzcv",
        ));
        insns.push((
            Inst::CondSet {
                rd: writable_xreg(5),
                cond: Cond::Hi,
            },
            "E5979F9A",
            "cset x5, hi",
        ));
        insns.push((
            Inst::VecRRR {
                rd: writable_vreg(21),
                rn: vreg(22),
                rm: vreg(23),
                alu_op: VecALUOp::UQAddScalar,
            },
            "D50EF77E",
            "uqadd d21, d22, d23",
        ));
        insns.push((
            Inst::VecRRR {
                rd: writable_vreg(21),
                rn: vreg(22),
                rm: vreg(23),
                alu_op: VecALUOp::SQAddScalar,
            },
            "D50EF75E",
            "sqadd d21, d22, d23",
        ));
        insns.push((
            Inst::VecRRR {
                rd: writable_vreg(21),
                rn: vreg(22),
                rm: vreg(23),
                alu_op: VecALUOp::UQSubScalar,
            },
            "D52EF77E",
            "uqsub d21, d22, d23",
        ));
        insns.push((
            Inst::VecRRR {
                rd: writable_vreg(21),
                rn: vreg(22),
                rm: vreg(23),
                alu_op: VecALUOp::SQSubScalar,
            },
            "D52EF75E",
            "sqsub d21, d22, d23",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: false,
                from_bits: 8,
                to_bits: 32,
            },
            "411C0053",
            "uxtb w1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: true,
                from_bits: 8,
                to_bits: 32,
            },
            "411C0013",
            "sxtb w1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: false,
                from_bits: 16,
                to_bits: 32,
            },
            "413C0053",
            "uxth w1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: true,
                from_bits: 16,
                to_bits: 32,
            },
            "413C0013",
            "sxth w1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: false,
                from_bits: 8,
                to_bits: 64,
            },
            "411C0053",
            "uxtb x1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: true,
                from_bits: 8,
                to_bits: 64,
            },
            "411C4093",
            "sxtb x1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: false,
                from_bits: 16,
                to_bits: 64,
            },
            "413C0053",
            "uxth x1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: true,
                from_bits: 16,
                to_bits: 64,
            },
            "413C4093",
            "sxth x1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: false,
                from_bits: 32,
                to_bits: 64,
            },
            "E103022A",
            "mov w1, w2",
        ));
        insns.push((
            Inst::Extend {
                rd: writable_xreg(1),
                rn: xreg(2),
                signed: true,
                from_bits: 32,
                to_bits: 64,
            },
            "417C4093",
            "sxtw x1, w2",
        ));

        insns.push((
            Inst::Jump {
                dest: BranchTarget::ResolvedOffset(64),
            },
            "10000014",
            "b 64",
        ));

        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Zero(xreg(8)),
            },
            "080200B4",
            "cbz x8, 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::NotZero(xreg(8)),
            },
            "080200B5",
            "cbnz x8, 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Eq),
            },
            "00020054",
            "b.eq 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Ne),
            },
            "01020054",
            "b.ne 64",
        ));

        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Hs),
            },
            "02020054",
            "b.hs 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Lo),
            },
            "03020054",
            "b.lo 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Mi),
            },
            "04020054",
            "b.mi 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Pl),
            },
            "05020054",
            "b.pl 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Vs),
            },
            "06020054",
            "b.vs 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Vc),
            },
            "07020054",
            "b.vc 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Hi),
            },
            "08020054",
            "b.hi 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Ls),
            },
            "09020054",
            "b.ls 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Ge),
            },
            "0A020054",
            "b.ge 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Lt),
            },
            "0B020054",
            "b.lt 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Gt),
            },
            "0C020054",
            "b.gt 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Le),
            },
            "0D020054",
            "b.le 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Al),
            },
            "0E020054",
            "b.al 64",
        ));
        insns.push((
            Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(64),
                kind: CondBrKind::Cond(Cond::Nv),
            },
            "0F020054",
            "b.nv 64",
        ));

        insns.push((
            Inst::CondBrLoweredCompound {
                taken: BranchTarget::ResolvedOffset(64),
                not_taken: BranchTarget::ResolvedOffset(128),
                kind: CondBrKind::Cond(Cond::Le),
            },
            "0D02005420000014",
            "b.le 64 ; b 128",
        ));

        insns.push((
            Inst::Call {
                dest: ExternalName::testcase("test0"),
                uses: Set::empty(),
                defs: Set::empty(),
                loc: SourceLoc::default(),
                opcode: Opcode::Call,
            },
            "00000094",
            "bl 0",
        ));

        insns.push((
            Inst::CallInd {
                rn: xreg(10),
                uses: Set::empty(),
                defs: Set::empty(),
                loc: SourceLoc::default(),
                opcode: Opcode::CallIndirect,
            },
            "40013FD6",
            "blr x10",
        ));

        insns.push((
            Inst::IndirectBr {
                rn: xreg(3),
                targets: vec![1, 2, 3],
            },
            "60001FD6",
            "br x3",
        ));

        insns.push((Inst::Brk, "000020D4", "brk #0"));

        insns.push((
            Inst::Adr {
                rd: writable_xreg(15),
                label: MemLabel::PCRel((1 << 20) - 4),
            },
            "EFFF7F10",
            "adr x15, pc+1048572",
        ));

        insns.push((
            Inst::FpuMove64 {
                rd: writable_vreg(8),
                rn: vreg(4),
            },
            "881CA40E",
            "mov v8.8b, v4.8b",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Abs32,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CFC3201E",
            "fabs s15, s30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Abs64,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CFC3601E",
            "fabs d15, d30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Neg32,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CF43211E",
            "fneg s15, s30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Neg64,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CF43611E",
            "fneg d15, d30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Sqrt32,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CFC3211E",
            "fsqrt s15, s30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Sqrt64,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CFC3611E",
            "fsqrt d15, d30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Cvt32To64,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CFC3221E",
            "fcvt d15, s30",
        ));

        insns.push((
            Inst::FpuRR {
                fpu_op: FPUOp1::Cvt64To32,
                rd: writable_vreg(15),
                rn: vreg(30),
            },
            "CF43621E",
            "fcvt s15, d30",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Add32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF2B3F1E",
            "fadd s15, s30, s31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Add64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF2B7F1E",
            "fadd d15, d30, d31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Sub32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF3B3F1E",
            "fsub s15, s30, s31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Sub64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF3B7F1E",
            "fsub d15, d30, d31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Mul32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF0B3F1E",
            "fmul s15, s30, s31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Mul64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF0B7F1E",
            "fmul d15, d30, d31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Div32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF1B3F1E",
            "fdiv s15, s30, s31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Div64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF1B7F1E",
            "fdiv d15, d30, d31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Max32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF4B3F1E",
            "fmax s15, s30, s31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Max64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF4B7F1E",
            "fmax d15, d30, d31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Min32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF5B3F1E",
            "fmin s15, s30, s31",
        ));

        insns.push((
            Inst::FpuRRR {
                fpu_op: FPUOp2::Min64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
            },
            "CF5B7F1E",
            "fmin d15, d30, d31",
        ));

        insns.push((
            Inst::FpuRRRR {
                fpu_op: FPUOp3::MAdd32,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
                ra: vreg(1),
            },
            "CF071F1F",
            "fmadd s15, s30, s31, s1",
        ));

        insns.push((
            Inst::FpuRRRR {
                fpu_op: FPUOp3::MAdd64,
                rd: writable_vreg(15),
                rn: vreg(30),
                rm: vreg(31),
                ra: vreg(1),
            },
            "CF075F1F",
            "fmadd d15, d30, d31, d1",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F32ToU32,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100391E",
            "fcvtzu w1, s4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F32ToU64,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100399E",
            "fcvtzu x1, s4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F32ToI32,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100381E",
            "fcvtzs w1, s4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F32ToI64,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100389E",
            "fcvtzs x1, s4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F64ToU32,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100791E",
            "fcvtzu w1, d4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F64ToU64,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100799E",
            "fcvtzu x1, d4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F64ToI32,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100781E",
            "fcvtzs w1, d4",
        ));

        insns.push((
            Inst::FpuToInt {
                op: FpuToIntOp::F64ToI64,
                rd: writable_xreg(1),
                rn: vreg(4),
            },
            "8100789E",
            "fcvtzs x1, d4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::U32ToF32,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100231E",
            "ucvtf s1, w4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::I32ToF32,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100221E",
            "scvtf s1, w4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::U32ToF64,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100631E",
            "ucvtf d1, w4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::I32ToF64,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100621E",
            "scvtf d1, w4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::U64ToF32,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100239E",
            "ucvtf s1, x4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::I64ToF32,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100229E",
            "scvtf s1, x4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::U64ToF64,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100639E",
            "ucvtf d1, x4",
        ));

        insns.push((
            Inst::IntToFpu {
                op: IntToFpuOp::I64ToF64,
                rd: writable_vreg(1),
                rn: xreg(4),
            },
            "8100629E",
            "scvtf d1, x4",
        ));

        insns.push((
            Inst::FpuCmp32 {
                rn: vreg(23),
                rm: vreg(24),
            },
            "E022381E",
            "fcmp s23, s24",
        ));

        insns.push((
            Inst::FpuCmp64 {
                rn: vreg(23),
                rm: vreg(24),
            },
            "E022781E",
            "fcmp d23, d24",
        ));

        insns.push((
            Inst::FpuLoad32 {
                rd: writable_vreg(16),
                mem: MemArg::RegScaled(xreg(8), xreg(9), F32),
                srcloc: None,
            },
            "107969BC",
            "ldr s16, [x8, x9, LSL #2]",
        ));

        insns.push((
            Inst::FpuLoad64 {
                rd: writable_vreg(16),
                mem: MemArg::RegScaled(xreg(8), xreg(9), F64),
                srcloc: None,
            },
            "107969FC",
            "ldr d16, [x8, x9, LSL #3]",
        ));

        insns.push((
            Inst::FpuLoad128 {
                rd: writable_vreg(16),
                mem: MemArg::RegScaled(xreg(8), xreg(9), I128),
                srcloc: None,
            },
            "1079E93C",
            "ldr q16, [x8, x9, LSL #4]",
        ));

        insns.push((
            Inst::FpuLoad32 {
                rd: writable_vreg(16),
                mem: MemArg::Label(MemLabel::PCRel(8)),
                srcloc: None,
            },
            "5000001C",
            "ldr s16, pc+8",
        ));

        insns.push((
            Inst::FpuLoad64 {
                rd: writable_vreg(16),
                mem: MemArg::Label(MemLabel::PCRel(8)),
                srcloc: None,
            },
            "5000005C",
            "ldr d16, pc+8",
        ));

        insns.push((
            Inst::FpuLoad128 {
                rd: writable_vreg(16),
                mem: MemArg::Label(MemLabel::PCRel(8)),
                srcloc: None,
            },
            "5000009C",
            "ldr q16, pc+8",
        ));

        insns.push((
            Inst::FpuStore32 {
                rd: vreg(16),
                mem: MemArg::RegScaled(xreg(8), xreg(9), F32),
                srcloc: None,
            },
            "107929BC",
            "str s16, [x8, x9, LSL #2]",
        ));

        insns.push((
            Inst::FpuStore64 {
                rd: vreg(16),
                mem: MemArg::RegScaled(xreg(8), xreg(9), F64),
                srcloc: None,
            },
            "107929FC",
            "str d16, [x8, x9, LSL #3]",
        ));

        insns.push((
            Inst::FpuStore128 {
                rd: vreg(16),
                mem: MemArg::RegScaled(xreg(8), xreg(9), I128),
                srcloc: None,
            },
            "1079A93C",
            "str q16, [x8, x9, LSL #4]",
        ));

        insns.push((
            Inst::LoadFpuConst32 {
                rd: writable_vreg(16),
                const_data: 1.0,
            },
            "5000001C020000140000803F",
            "ldr s16, pc+8 ; b 8 ; data.f32 1",
        ));

        insns.push((
            Inst::LoadFpuConst64 {
                rd: writable_vreg(16),
                const_data: 1.0,
            },
            "5000005C03000014000000000000F03F",
            "ldr d16, pc+8 ; b 12 ; data.f64 1",
        ));

        insns.push((
            Inst::FpuCSel32 {
                rd: writable_vreg(1),
                rn: vreg(2),
                rm: vreg(3),
                cond: Cond::Hi,
            },
            "418C231E",
            "fcsel s1, s2, s3, hi",
        ));

        insns.push((
            Inst::FpuCSel64 {
                rd: writable_vreg(1),
                rn: vreg(2),
                rm: vreg(3),
                cond: Cond::Eq,
            },
            "410C631E",
            "fcsel d1, d2, d3, eq",
        ));

        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Minus32,
            },
            "1743251E",
            "frintm s23, s24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Minus64,
            },
            "1743651E",
            "frintm d23, d24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Plus32,
            },
            "17C3241E",
            "frintp s23, s24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Plus64,
            },
            "17C3641E",
            "frintp d23, d24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Zero32,
            },
            "17C3251E",
            "frintz s23, s24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Zero64,
            },
            "17C3651E",
            "frintz d23, d24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Nearest32,
            },
            "1743241E",
            "frintn s23, s24",
        ));
        insns.push((
            Inst::FpuRound {
                rd: writable_vreg(23),
                rn: vreg(24),
                op: FpuRoundMode::Nearest64,
            },
            "1743641E",
            "frintn d23, d24",
        ));

        let rru = create_reg_universe(&settings::Flags::new(settings::builder()));
        for (insn, expected_encoding, expected_printing) in insns {
            println!(
                "AArch64: {:?}, {}, {}",
                insn, expected_encoding, expected_printing
            );

            // Check the printed text is as expected.
            let actual_printing = insn.show_rru(Some(&rru));
            assert_eq!(expected_printing, actual_printing);

            // Check the encoding is as expected.
            let text_size = {
                let mut code_sec = MachSectionSize::new(0);
                insn.emit(&mut code_sec);
                code_sec.size()
            };

            let mut sink = test_utils::TestCodeSink::new();
            let mut sections = MachSections::new();
            let code_idx = sections.add_section(0, text_size);
            let code_sec = sections.get_section(code_idx);
            insn.emit(code_sec);
            sections.emit(&mut sink);
            let actual_encoding = &sink.stringify();
            assert_eq!(expected_encoding, actual_encoding);
        }
    }

    #[test]
    fn test_cond_invert() {
        for cond in vec![
            Cond::Eq,
            Cond::Ne,
            Cond::Hs,
            Cond::Lo,
            Cond::Mi,
            Cond::Pl,
            Cond::Vs,
            Cond::Vc,
            Cond::Hi,
            Cond::Ls,
            Cond::Ge,
            Cond::Lt,
            Cond::Gt,
            Cond::Le,
            Cond::Al,
            Cond::Nv,
        ]
        .into_iter()
        {
            assert_eq!(cond.invert().invert(), cond);
        }
    }
}
