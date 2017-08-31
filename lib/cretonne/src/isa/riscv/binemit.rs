//! Emitting binary RISC-V machine code.

use binemit::{CodeSink, Reloc, bad_encoding};
use ir::{Function, Inst, InstructionData};
use isa::RegUnit;
use predicates::is_signed_int;
use regalloc::RegDiversions;

include!(concat!(env!("OUT_DIR"), "/binemit-riscv.rs"));

/// RISC-V relocation kinds.
pub enum RelocKind {
    /// A jal call to a function.
    Call,
}

pub static RELOC_NAMES: [&'static str; 1] = ["Call"];

impl Into<Reloc> for RelocKind {
    fn into(self) -> Reloc {
        Reloc(self as u16)
    }
}

/// R-type instructions.
///
///   31     24  19  14     11 6
///   funct7 rs2 rs1 funct3 rd opcode
///       25  20  15     12  7      0
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5) | (funct7 << 8)`.
fn put_r<CS: CodeSink + ?Sized>(bits: u16, rs1: RegUnit, rs2: RegUnit, rd: RegUnit, sink: &mut CS) {
    let bits = bits as u32;
    let opcode5 = bits & 0x1f;
    let funct3 = (bits >> 5) & 0x7;
    let funct7 = (bits >> 8) & 0x7f;
    let rs1 = rs1 as u32 & 0x1f;
    let rs2 = rs2 as u32 & 0x1f;
    let rd = rd as u32 & 0x1f;

    // 0-6: opcode
    let mut i = 0x3;
    i |= opcode5 << 2;
    i |= rd << 7;
    i |= funct3 << 12;
    i |= rs1 << 15;
    i |= rs2 << 20;
    i |= funct7 << 25;

    sink.put4(i);
}

/// R-type instructions with a shift amount instead of rs2.
///
///   31     25    19  14     11 6
///   funct7 shamt rs1 funct3 rd opcode
///       25    20  15     12  7      0
///
/// Both funct7 and shamt contribute to bit 25. In RV64, shamt uses it for shifts > 31.
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5) | (funct7 << 8)`.
fn put_rshamt<CS: CodeSink + ?Sized>(
    bits: u16,
    rs1: RegUnit,
    shamt: i64,
    rd: RegUnit,
    sink: &mut CS,
) {
    let bits = bits as u32;
    let opcode5 = bits & 0x1f;
    let funct3 = (bits >> 5) & 0x7;
    let funct7 = (bits >> 8) & 0x7f;
    let rs1 = rs1 as u32 & 0x1f;
    let shamt = shamt as u32 & 0x3f;
    let rd = rd as u32 & 0x1f;

    // 0-6: opcode
    let mut i = 0x3;
    i |= opcode5 << 2;
    i |= rd << 7;
    i |= funct3 << 12;
    i |= rs1 << 15;
    i |= shamt << 20;
    i |= funct7 << 25;

    sink.put4(i);
}

/// I-type instructions.
///
///   31  19  14     11 6
///   imm rs1 funct3 rd opcode
///    20  15     12  7      0
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5)`
fn put_i<CS: CodeSink + ?Sized>(bits: u16, rs1: RegUnit, imm: i64, rd: RegUnit, sink: &mut CS) {
    let bits = bits as u32;
    let opcode5 = bits & 0x1f;
    let funct3 = (bits >> 5) & 0x7;
    let rs1 = rs1 as u32 & 0x1f;
    let rd = rd as u32 & 0x1f;

    // 0-6: opcode
    let mut i = 0x3;
    i |= opcode5 << 2;
    i |= rd << 7;
    i |= funct3 << 12;
    i |= rs1 << 15;
    i |= (imm << 20) as u32;

    sink.put4(i);
}

/// U-type instructions.
///
///   31  11 6
///   imm rd opcode
///    12  7      0
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5)`
fn put_u<CS: CodeSink + ?Sized>(bits: u16, imm: i64, rd: RegUnit, sink: &mut CS) {
    let bits = bits as u32;
    let opcode5 = bits & 0x1f;
    let rd = rd as u32 & 0x1f;

    // 0-6: opcode
    let mut i = 0x3;
    i |= opcode5 << 2;
    i |= rd << 7;
    i |= imm as u32 & 0xfffff000;

    sink.put4(i);
}

/// SB-type branch instructions.
///
///   31  24  19  14     11  6
///   imm rs2 rs1 funct3 imm opcode
///    25  20  15     12   7      0
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5)`
fn put_sb<CS: CodeSink + ?Sized>(bits: u16, imm: i64, rs1: RegUnit, rs2: RegUnit, sink: &mut CS) {
    let bits = bits as u32;
    let opcode5 = bits & 0x1f;
    let funct3 = (bits >> 5) & 0x7;
    let rs1 = rs1 as u32 & 0x1f;
    let rs2 = rs2 as u32 & 0x1f;

    assert!(is_signed_int(imm, 13, 1), "SB out of range {:#x}", imm);
    let imm = imm as u32;

    // 0-6: opcode
    let mut i = 0x3;
    i |= opcode5 << 2;
    i |= funct3 << 12;
    i |= rs1 << 15;
    i |= rs2 << 20;

    // The displacement is completely hashed up.
    i |= ((imm >> 11) & 0x1) << 7;
    i |= ((imm >> 1) & 0xf) << 8;
    i |= ((imm >> 5) & 0x3f) << 25;
    i |= ((imm >> 12) & 0x1) << 31;

    sink.put4(i);
}

/// UJ-type jump instructions.
///
///   31  11 6
///   imm rd opcode
///    12  7      0
///
/// Encoding bits: `opcode[6:2]`
fn put_uj<CS: CodeSink + ?Sized>(bits: u16, imm: i64, rd: RegUnit, sink: &mut CS) {
    let bits = bits as u32;
    let opcode5 = bits & 0x1f;
    let rd = rd as u32 & 0x1f;

    assert!(is_signed_int(imm, 21, 1), "UJ out of range {:#x}", imm);
    let imm = imm as u32;

    // 0-6: opcode
    let mut i = 0x3;
    i |= opcode5 << 2;
    i |= rd << 7;

    // The displacement is completely hashed up.
    i |= imm & 0xff000;
    i |= ((imm >> 11) & 0x1) << 20;
    i |= ((imm >> 1) & 0x3ff) << 21;
    i |= ((imm >> 20) & 0x1) << 31;

    sink.put4(i);
}
