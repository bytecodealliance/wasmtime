//! Emitting binary RISC-V machine code.

use binemit::{CodeSink, bad_encoding};
use ir::{Function, Inst, InstructionData};
use isa::RegUnit;

include!(concat!(env!("OUT_DIR"), "/binemit-riscv.rs"));

/// R-type instructions.
///
///   31     24  19  14     11 6
///   funct7 rs2 rs1 funct3 rd opcode
///       25  20  15     12  7      0
///
/// Encoding bits: `opcode[6:2] | (funct3 << 5) | (funct7 << 8)`.
fn put_r<CS: CodeSink + ?Sized>(bits: u16,
                                rs1: RegUnit,
                                rs2: RegUnit,
                                rd: RegUnit,
                                sink: &mut CS) {
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
fn put_rshamt<CS: CodeSink + ?Sized>(bits: u16,
                                     rs1: RegUnit,
                                     shamt: i64,
                                     rd: RegUnit,
                                     sink: &mut CS) {
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

fn recipe_r<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Binary { args, .. } = func.dfg[inst] {
        put_r(func.encodings[inst].bits(),
              func.locations[args[0]].unwrap_reg(),
              func.locations[args[1]].unwrap_reg(),
              func.locations[func.dfg.first_result(inst)].unwrap_reg(),
              sink);
    } else {
        panic!("Expected Binary format: {:?}", func.dfg[inst]);
    }
}

fn recipe_ricmp<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::IntCompare { args, .. } = func.dfg[inst] {
        put_r(func.encodings[inst].bits(),
              func.locations[args[0]].unwrap_reg(),
              func.locations[args[1]].unwrap_reg(),
              func.locations[func.dfg.first_result(inst)].unwrap_reg(),
              sink);
    } else {
        panic!("Expected IntCompare format: {:?}", func.dfg[inst]);
    }
}

fn recipe_rshamt<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::BinaryImm { arg, imm, .. } = func.dfg[inst] {
        put_rshamt(func.encodings[inst].bits(),
                   func.locations[arg].unwrap_reg(),
                   imm.into(),
                   func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                   sink);
    } else {
        panic!("Expected BinaryImm format: {:?}", func.dfg[inst]);
    }
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

fn recipe_i<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::BinaryImm { arg, imm, .. } = func.dfg[inst] {
        put_i(func.encodings[inst].bits(),
              func.locations[arg].unwrap_reg(),
              imm.into(),
              func.locations[func.dfg.first_result(inst)].unwrap_reg(),
              sink);
    } else {
        panic!("Expected BinaryImm format: {:?}", func.dfg[inst]);
    }
}

fn recipe_iicmp<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::IntCompareImm { arg, imm, .. } = func.dfg[inst] {
        put_i(func.encodings[inst].bits(),
              func.locations[arg].unwrap_reg(),
              imm.into(),
              func.locations[func.dfg.first_result(inst)].unwrap_reg(),
              sink);
    } else {
        panic!("Expected IntCompareImm format: {:?}", func.dfg[inst]);
    }
}

fn recipe_iret<CS: CodeSink + ?Sized>(_func: &Function, _inst: Inst, _sink: &mut CS) {
    unimplemented!()
}
