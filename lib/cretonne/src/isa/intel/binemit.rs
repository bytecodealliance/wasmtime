//! Emitting binary Intel machine code.

use binemit::{CodeSink, Reloc, bad_encoding};
use ir::{Function, Inst, Ebb, InstructionData, Opcode};
use isa::{RegUnit, StackRef, StackBase, StackBaseMask};
use regalloc::RegDiversions;
use super::registers::RU;

include!(concat!(env!("OUT_DIR"), "/binemit-intel.rs"));

/// Intel relocations.
pub enum RelocKind {
    /// A 4-byte relative function reference. Based from relocation + 4 bytes.
    PCRel4,

    /// A 4-byte absolute function reference.
    Abs4,

    /// An 8-byte absolute function reference.
    Abs8,
}

pub static RELOC_NAMES: [&'static str; 3] = ["PCRel4", "Abs4", "Abs8"];

impl Into<Reloc> for RelocKind {
    fn into(self) -> Reloc {
        Reloc(self as u16)
    }
}

// Convert a stack base to the corresponding register.
fn stk_base(base: StackBase) -> RegUnit {
    let ru = match base {
        StackBase::SP => RU::rsp,
        StackBase::FP => RU::rbp,
        StackBase::Zone => unimplemented!(),
    };
    ru as RegUnit
}

// Mandatory prefix bytes for Mp* opcodes.
const PREFIX: [u8; 3] = [0x66, 0xf3, 0xf2];

// Second byte for three-byte opcodes for mm=0b10 and mm=0b11.
const OP3_BYTE2: [u8; 2] = [0x38, 0x3a];

// A REX prefix with no bits set: 0b0100WRXB.
const BASE_REX: u8 = 0b0100_0000;

// Create a single-register REX prefix, setting the B bit to bit 3 of the register.
// This is used for instructions that encode a register in the low 3 bits of the opcode and for
// instructions that use the ModR/M `reg` field for something else.
fn rex1(reg_b: RegUnit) -> u8 {
    let b = ((reg_b >> 3) & 1) as u8;
    BASE_REX | b
}

// Create a dual-register REX prefix, setting:
//
// REX.B = bit 3 of r/m register, or SIB base register when a SIB byte is present.
// REX.R = bit 3 of reg register.
fn rex2(rm: RegUnit, reg: RegUnit) -> u8 {
    let b = ((rm >> 3) & 1) as u8;
    let r = ((reg >> 3) & 1) as u8;
    BASE_REX | b | (r << 2)
}

// Emit a REX prefix.
//
// The R, X, and B bits are computed from registers using the functions above. The W bit is
// extracted from `bits`.
fn rex_prefix<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(rex & 0xf8, BASE_REX);
    let w = ((bits >> 15) & 1) as u8;
    sink.put1(rex | (w << 3));
}

// Emit a single-byte opcode with no REX prefix.
fn put_op1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8f00, 0, "Invalid encoding bits for Op1*");
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Op1 encoding");
    sink.put1(bits as u8);
}

// Emit a single-byte opcode with REX prefix.
fn put_rexop1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0f00, 0, "Invalid encoding bits for Op1*");
    rex_prefix(bits, rex, sink);
    sink.put1(bits as u8);
}

// Emit two-byte opcode: 0F XX
fn put_op2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8f00, 0x0400, "Invalid encoding bits for Op2*");
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Op2 encoding");
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Emit two-byte opcode: 0F XX with REX prefix.
fn put_rexop2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0f00, 0x0400, "Invalid encoding bits for RexOp2*");
    rex_prefix(bits, rex, sink);
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Emit single-byte opcode with mandatory prefix.
fn put_mp1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8c00, 0, "Invalid encoding bits for Mp1*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Mp1 encoding");
    sink.put1(bits as u8);
}

// Emit single-byte opcode with mandatory prefix and REX.
fn put_rexmp1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0c00, 0, "Invalid encoding bits for Mp1*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
    rex_prefix(bits, rex, sink);
    sink.put1(bits as u8);
}

// Emit two-byte opcode (0F XX) with mandatory prefix.
fn put_mp2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8c00, 0x0400, "Invalid encoding bits for Mp2*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Mp2 encoding");
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Emit two-byte opcode (0F XX) with mandatory prefix and REX.
fn put_rexmp2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0c00, 0x0400, "Invalid encoding bits for Mp2*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
    rex_prefix(bits, rex, sink);
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Emit three-byte opcode (0F 3[8A] XX) with mandatory prefix.
fn put_mp3<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8800, 0x0800, "Invalid encoding bits for Mp3*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Mp3 encoding");
    let mm = (bits >> 10) & 3;
    sink.put1(0x0f);
    sink.put1(OP3_BYTE2[(mm - 2) as usize]);
    sink.put1(bits as u8);
}

// Emit three-byte opcode (0F 3[8A] XX) with mandatory prefix and REX
fn put_rexmp3<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0800, 0x0800, "Invalid encoding bits for Mp3*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
    rex_prefix(bits, rex, sink);
    let mm = (bits >> 10) & 3;
    sink.put1(0x0f);
    sink.put1(OP3_BYTE2[(mm - 2) as usize]);
    sink.put1(bits as u8);
}

/// Emit a ModR/M byte for reg-reg operands.
fn modrm_rr<CS: CodeSink + ?Sized>(rm: RegUnit, reg: RegUnit, sink: &mut CS) {
    let reg = reg as u8 & 7;
    let rm = rm as u8 & 7;
    let mut b = 0b11000000;
    b |= reg << 3;
    b |= rm;
    sink.put1(b);
}

/// Emit a ModR/M byte where the reg bits are part of the opcode.
fn modrm_r_bits<CS: CodeSink + ?Sized>(rm: RegUnit, bits: u16, sink: &mut CS) {
    let reg = (bits >> 12) as u8 & 7;
    let rm = rm as u8 & 7;
    let mut b = 0b11000000;
    b |= reg << 3;
    b |= rm;
    sink.put1(b);
}

/// Emit a mode 00 ModR/M byte. This is a register-indirect addressing mode with no offset.
/// Registers %rsp and %rbp are invalid for `rm`, %rsp indicates a SIB byte, and %rbp indicates an
/// absolute immediate 32-bit address.
fn modrm_rm<CS: CodeSink + ?Sized>(rm: RegUnit, reg: RegUnit, sink: &mut CS) {
    let reg = reg as u8 & 7;
    let rm = rm as u8 & 7;
    let mut b = 0b00000000;
    b |= reg << 3;
    b |= rm;
    sink.put1(b);
}

/// Emit a mode 01 ModR/M byte. This is a register-indirect addressing mode with 8-bit
/// displacement.
/// Register %rsp is invalid for `rm`. It indicates the presence of a SIB byte.
fn modrm_disp8<CS: CodeSink + ?Sized>(rm: RegUnit, reg: RegUnit, sink: &mut CS) {
    let reg = reg as u8 & 7;
    let rm = rm as u8 & 7;
    let mut b = 0b01000000;
    b |= reg << 3;
    b |= rm;
    sink.put1(b);
}

/// Emit a mode 10 ModR/M byte. This is a register-indirect addressing mode with 32-bit
/// displacement.
/// Register %rsp is invalid for `rm`. It indicates the presence of a SIB byte.
fn modrm_disp32<CS: CodeSink + ?Sized>(rm: RegUnit, reg: RegUnit, sink: &mut CS) {
    let reg = reg as u8 & 7;
    let rm = rm as u8 & 7;
    let mut b = 0b10000000;
    b |= reg << 3;
    b |= rm;
    sink.put1(b);
}

/// Emit a mode 10 ModR/M byte indicating that a SIB byte is present.
fn modrm_sib_disp32<CS: CodeSink + ?Sized>(reg: RegUnit, sink: &mut CS) {
    modrm_disp32(0b100, reg, sink);
}

/// Emit a SIB byte with a base register and no scale+index.
fn sib_noindex<CS: CodeSink + ?Sized>(base: RegUnit, sink: &mut CS) {
    let base = base as u8 & 7;
    // SIB        SS_III_BBB.
    let mut b = 0b00_100_000;
    b |= base;
    sink.put1(b);
}

/// Emit a single-byte branch displacement to `destination`.
fn disp1<CS: CodeSink + ?Sized>(destination: Ebb, func: &Function, sink: &mut CS) {
    let delta = func.offsets[destination].wrapping_sub(sink.offset() + 1);
    sink.put1(delta as u8);
}

/// Emit a single-byte branch displacement to `destination`.
fn disp4<CS: CodeSink + ?Sized>(destination: Ebb, func: &Function, sink: &mut CS) {
    let delta = func.offsets[destination].wrapping_sub(sink.offset() + 4);
    sink.put4(delta);
}
