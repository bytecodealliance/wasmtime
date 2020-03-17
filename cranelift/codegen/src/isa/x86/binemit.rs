//! Emitting binary x86 machine code.

use super::enc_tables::{needs_offset, needs_sib_byte};
use super::registers::RU;
use crate::binemit::{bad_encoding, CodeSink, Reloc};
use crate::ir::condcodes::{CondCode, FloatCC, IntCC};
use crate::ir::{
    Block, Constant, ExternalName, Function, Inst, InstructionData, JumpTable, LibCall, Opcode,
    TrapCode,
};
use crate::isa::{RegUnit, StackBase, StackBaseMask, StackRef, TargetIsa};
use crate::regalloc::RegDiversions;
use cranelift_codegen_shared::isa::x86::EncodingBits;

include!(concat!(env!("OUT_DIR"), "/binemit-x86.rs"));

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

// Create a three-register REX prefix, setting:
//
// REX.B = bit 3 of r/m register, or SIB base register when a SIB byte is present.
// REX.R = bit 3 of reg register.
// REX.X = bit 3 of SIB index register.
fn rex3(rm: RegUnit, reg: RegUnit, index: RegUnit) -> u8 {
    let b = ((rm >> 3) & 1) as u8;
    let r = ((reg >> 3) & 1) as u8;
    let x = ((index >> 3) & 1) as u8;
    BASE_REX | b | (x << 1) | (r << 2)
}

/// Encode the RXBR' bits of the EVEX P0 byte. For an explanation of these bits, see section 2.6.1
/// in the Intel Software Development Manual, volume 2A. These bits can be used by different
/// addressing modes (see section 2.6.2), requiring different `vex*` functions than this one.
fn evex2(rm: RegUnit, reg: RegUnit) -> u8 {
    let b = (!(rm >> 3) & 1) as u8;
    let x = (!(rm >> 4) & 1) as u8;
    let r = (!(reg >> 3) & 1) as u8;
    let r_ = (!(reg >> 4) & 1) as u8;
    0x00 | r_ | (b << 1) | (x << 2) | (r << 3)
}

/// Determines whether a REX prefix should be emitted.
#[inline]
fn needs_rex(bits: u16, rex: u8) -> bool {
    rex != BASE_REX || EncodingBits::from(bits).rex_w() == 1
}

// Emit a REX prefix.
//
// The R, X, and B bits are computed from registers using the functions above. The W bit is
// extracted from `bits`.
fn rex_prefix<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(rex & 0xf8, BASE_REX);
    let w = EncodingBits::from(bits).rex_w();
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
    debug_assert_eq!(bits & 0x0f00, 0, "Invalid encoding bits for RexOp1*");
    rex_prefix(bits, rex, sink);
    sink.put1(bits as u8);
}

/// Emit a single-byte opcode with inferred REX prefix.
fn put_dynrexop1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0f00, 0, "Invalid encoding bits for DynRexOp1*");
    if needs_rex(bits, rex) {
        rex_prefix(bits, rex, sink);
    }
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

/// Emit two-byte opcode: 0F XX with inferred REX prefix.
fn put_dynrexop2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(
        bits & 0x0f00,
        0x0400,
        "Invalid encoding bits for DynRexOp2*"
    );
    if needs_rex(bits, rex) {
        rex_prefix(bits, rex, sink);
    }
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Emit single-byte opcode with mandatory prefix.
fn put_mp1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8c00, 0, "Invalid encoding bits for Mp1*");
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Mp1 encoding");
    sink.put1(bits as u8);
}

// Emit single-byte opcode with mandatory prefix and REX.
fn put_rexmp1<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0c00, 0, "Invalid encoding bits for RexMp1*");
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    rex_prefix(bits, rex, sink);
    sink.put1(bits as u8);
}

// Emit two-byte opcode (0F XX) with mandatory prefix.
fn put_mp2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8c00, 0x0400, "Invalid encoding bits for Mp2*");
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Mp2 encoding");
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Emit two-byte opcode (0F XX) with mandatory prefix and REX.
fn put_rexmp2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0c00, 0x0400, "Invalid encoding bits for RexMp2*");
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    rex_prefix(bits, rex, sink);
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

/// Emit two-byte opcode (0F XX) with mandatory prefix and inferred REX.
fn put_dynrexmp2<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(
        bits & 0x0c00,
        0x0400,
        "Invalid encoding bits for DynRexMp2*"
    );
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    if needs_rex(bits, rex) {
        rex_prefix(bits, rex, sink);
    }
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

/// Emit three-byte opcode (0F 3[8A] XX) with mandatory prefix.
fn put_mp3<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x8800, 0x0800, "Invalid encoding bits for Mp3*");
    debug_assert_eq!(rex, BASE_REX, "Invalid registers for REX-less Mp3 encoding");
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    sink.put1(0x0f);
    sink.put1(OP3_BYTE2[(enc.mm() - 2) as usize]);
    sink.put1(bits as u8);
}

/// Emit three-byte opcode (0F 3[8A] XX) with mandatory prefix and REX
fn put_rexmp3<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0800, 0x0800, "Invalid encoding bits for RexMp3*");
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    rex_prefix(bits, rex, sink);
    sink.put1(0x0f);
    sink.put1(OP3_BYTE2[(enc.mm() - 2) as usize]);
    sink.put1(bits as u8);
}

/// Emit three-byte opcode (0F 3[8A] XX) with mandatory prefix and an inferred REX prefix.
fn put_dynrexmp3<CS: CodeSink + ?Sized>(bits: u16, rex: u8, sink: &mut CS) {
    debug_assert_eq!(
        bits & 0x0800,
        0x0800,
        "Invalid encoding bits for DynRexMp3*"
    );
    let enc = EncodingBits::from(bits);
    sink.put1(PREFIX[(enc.pp() - 1) as usize]);
    if needs_rex(bits, rex) {
        rex_prefix(bits, rex, sink);
    }
    sink.put1(0x0f);
    sink.put1(OP3_BYTE2[(enc.mm() - 2) as usize]);
    sink.put1(bits as u8);
}

/// Defines the EVEX context for the `L'`, `L`, and `b` bits (bits 6:4 of EVEX P2 byte). Table 2-36 in
/// section 2.6.10 (Intel Software Development Manual, volume 2A) describes how these bits can be
/// used together for certain classes of instructions; i.e., special care should be taken to ensure
/// that instructions use an applicable correct `EvexContext`. Table 2-39 contains cases where
/// opcodes can result in an #UD.
#[allow(dead_code)]
enum EvexContext {
    RoundingRegToRegFP {
        rc: EvexRoundingControl,
    },
    NoRoundingFP {
        sae: bool,
        length: EvexVectorLength,
    },
    MemoryOp {
        broadcast: bool,
        length: EvexVectorLength,
    },
    Other {
        length: EvexVectorLength,
    },
}

impl EvexContext {
    /// Encode the `L'`, `L`, and `b` bits (bits 6:4 of EVEX P2 byte) for merging with the P2 byte.
    fn bits(&self) -> u8 {
        match self {
            Self::RoundingRegToRegFP { rc } => 0b001 | rc.bits() << 1,
            Self::NoRoundingFP { sae, length } => (*sae as u8) | length.bits() << 1,
            Self::MemoryOp { broadcast, length } => (*broadcast as u8) | length.bits() << 1,
            Self::Other { length } => length.bits() << 1,
        }
    }
}

/// The EVEX format allows choosing a vector length in the `L'` and `L` bits; see `EvexContext`.
enum EvexVectorLength {
    V128,
    V256,
    V512,
}

impl EvexVectorLength {
    /// Encode the `L'` and `L` bits for merging with the P2 byte.
    fn bits(&self) -> u8 {
        match self {
            Self::V128 => 0b00,
            Self::V256 => 0b01,
            Self::V512 => 0b10,
            // 0b11 is reserved (#UD).
        }
    }
}

/// The EVEX format allows defining rounding control in the `L'` and `L` bits; see `EvexContext`.
enum EvexRoundingControl {
    RNE,
    RD,
    RU,
    RZ,
}

impl EvexRoundingControl {
    /// Encode the `L'` and `L` bits for merging with the P2 byte.
    fn bits(&self) -> u8 {
        match self {
            Self::RNE => 0b00,
            Self::RD => 0b01,
            Self::RU => 0b10,
            Self::RZ => 0b11,
        }
    }
}

/// Defines the EVEX masking behavior; masking support is described in section 2.6.4 of the Intel
/// Software Development Manual, volume 2A.
#[allow(dead_code)]
enum EvexMasking {
    None,
    Merging { k: u8 },
    Zeroing { k: u8 },
}

impl EvexMasking {
    /// Encode the `z` bit for merging with the P2 byte.
    fn z_bit(&self) -> u8 {
        match self {
            Self::None | Self::Merging { .. } => 0,
            Self::Zeroing { .. } => 1,
        }
    }

    /// Encode the `aaa` bits for merging with the P2 byte.
    fn aaa_bits(&self) -> u8 {
        match self {
            Self::None => 0b000,
            Self::Merging { k } | Self::Zeroing { k } => {
                debug_assert!(*k <= 7);
                *k
            }
        }
    }
}

/// Encode an EVEX prefix, including the instruction opcode. To match the current recipe
/// convention, the ModR/M byte is written separately in the recipe. This EVEX encoding function
/// only encodes the `reg` (operand 1), `vvvv` (operand 2), `rm` (operand 3) form; other forms are
/// possible (see section 2.6.2, Intel Software Development Manual, volume 2A), requiring
/// refactoring of this function or separate functions for each form (e.g. as for the REX prefix).
fn put_evex<CS: CodeSink + ?Sized>(
    bits: u16,
    reg: RegUnit,
    vvvvv: RegUnit,
    rm: RegUnit,
    context: EvexContext,
    masking: EvexMasking,
    sink: &mut CS,
) {
    let enc = EncodingBits::from(bits);

    // EVEX prefix.
    sink.put1(0x62);

    debug_assert!(enc.mm() < 0b100);
    let mut p0 = enc.mm() & 0b11;
    p0 |= evex2(rm, reg) << 4; // bits 3:2 are always unset
    sink.put1(p0);

    let mut p1 = enc.pp() | 0b100; // bit 2 is always set
    p1 |= (!(vvvvv as u8) & 0b1111) << 3;
    p1 |= (enc.rex_w() & 0b1) << 7;
    sink.put1(p1);

    let mut p2 = masking.aaa_bits();
    p2 |= (!(vvvvv as u8 >> 4) & 0b1) << 3;
    p2 |= context.bits() << 4;
    p2 |= masking.z_bit() << 7;
    sink.put1(p2);

    // Opcode
    sink.put1(enc.opcode_byte());

    // ModR/M byte placed in recipe
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

/// Emit a mode 00 Mod/RM byte, with a rip-relative displacement in 64-bit mode. Effective address
/// is calculated by adding displacement to 64-bit rip of next instruction. See intel Sw dev manual
/// section 2.2.1.6.
fn modrm_riprel<CS: CodeSink + ?Sized>(reg: RegUnit, sink: &mut CS) {
    modrm_rm(0b101, reg, sink)
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

/// Emit a mode 00 ModR/M with a 100 RM indicating a SIB byte is present.
fn modrm_sib<CS: CodeSink + ?Sized>(reg: RegUnit, sink: &mut CS) {
    modrm_rm(0b100, reg, sink);
}

/// Emit a mode 01 ModR/M with a 100 RM indicating a SIB byte and 8-bit
/// displacement are present.
fn modrm_sib_disp8<CS: CodeSink + ?Sized>(reg: RegUnit, sink: &mut CS) {
    modrm_disp8(0b100, reg, sink);
}

/// Emit a mode 10 ModR/M with a 100 RM indicating a SIB byte and 32-bit
/// displacement are present.
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

/// Emit a SIB byte with a scale, base, and index.
fn sib<CS: CodeSink + ?Sized>(scale: u8, index: RegUnit, base: RegUnit, sink: &mut CS) {
    // SIB        SS_III_BBB.
    debug_assert_eq!(scale & !0x03, 0, "Scale out of range");
    let scale = scale & 3;
    let index = index as u8 & 7;
    let base = base as u8 & 7;
    let b: u8 = (scale << 6) | (index << 3) | base;
    sink.put1(b);
}

/// Get the low 4 bits of an opcode for an integer condition code.
///
/// Add this offset to a base opcode for:
///
/// ---- 0x70: Short conditional branch.
/// 0x0f 0x80: Long conditional branch.
/// 0x0f 0x90: SetCC.
///
fn icc2opc(cond: IntCC) -> u16 {
    use crate::ir::condcodes::IntCC::*;
    match cond {
        Overflow => 0x0,
        NotOverflow => 0x1,
        UnsignedLessThan => 0x2,
        UnsignedGreaterThanOrEqual => 0x3,
        Equal => 0x4,
        NotEqual => 0x5,
        UnsignedLessThanOrEqual => 0x6,
        UnsignedGreaterThan => 0x7,
        // 0x8 = Sign.
        // 0x9 = !Sign.
        // 0xa = Parity even.
        // 0xb = Parity odd.
        SignedLessThan => 0xc,
        SignedGreaterThanOrEqual => 0xd,
        SignedLessThanOrEqual => 0xe,
        SignedGreaterThan => 0xf,
    }
}

/// Get the low 4 bits of an opcode for a floating point condition code.
///
/// The ucomiss/ucomisd instructions set the FLAGS bits CF/PF/CF like this:
///
///    ZPC OSA
/// UN 111 000
/// GT 000 000
/// LT 001 000
/// EQ 100 000
///
/// Not all floating point condition codes are supported.
fn fcc2opc(cond: FloatCC) -> u16 {
    use crate::ir::condcodes::FloatCC::*;
    match cond {
        Ordered                    => 0xb, // EQ|LT|GT => *np (P=0)
        Unordered                  => 0xa, // UN       => *p  (P=1)
        OrderedNotEqual            => 0x5, // LT|GT    => *ne (Z=0),
        UnorderedOrEqual           => 0x4, // UN|EQ    => *e  (Z=1)
        GreaterThan                => 0x7, // GT       => *a  (C=0&Z=0)
        GreaterThanOrEqual         => 0x3, // GT|EQ    => *ae (C=0)
        UnorderedOrLessThan        => 0x2, // UN|LT    => *b  (C=1)
        UnorderedOrLessThanOrEqual => 0x6, // UN|LT|EQ => *be (Z=1|C=1)
        Equal |                            // EQ
        NotEqual |                         // UN|LT|GT
        LessThan |                         // LT
        LessThanOrEqual |                  // LT|EQ
        UnorderedOrGreaterThan |           // UN|GT
        UnorderedOrGreaterThanOrEqual      // UN|GT|EQ
        => panic!("{} not supported", cond),
    }
}

/// Emit a single-byte branch displacement to `destination`.
fn disp1<CS: CodeSink + ?Sized>(destination: Block, func: &Function, sink: &mut CS) {
    let delta = func.offsets[destination].wrapping_sub(sink.offset() + 1);
    sink.put1(delta as u8);
}

/// Emit a four-byte branch displacement to `destination`.
fn disp4<CS: CodeSink + ?Sized>(destination: Block, func: &Function, sink: &mut CS) {
    let delta = func.offsets[destination].wrapping_sub(sink.offset() + 4);
    sink.put4(delta);
}

/// Emit a four-byte displacement to jump table `jt`.
fn jt_disp4<CS: CodeSink + ?Sized>(jt: JumpTable, func: &Function, sink: &mut CS) {
    let delta = func.jt_offsets[jt].wrapping_sub(sink.offset() + 4);
    sink.put4(delta);
    sink.reloc_jt(Reloc::X86PCRelRodata4, jt);
}

/// Emit a four-byte displacement to `constant`.
fn const_disp4<CS: CodeSink + ?Sized>(constant: Constant, func: &Function, sink: &mut CS) {
    let offset = func.dfg.constants.get_offset(constant);
    let delta = offset.wrapping_sub(sink.offset() + 4);
    sink.put4(delta);
    sink.reloc_constant(Reloc::X86PCRelRodata4, offset);
}
