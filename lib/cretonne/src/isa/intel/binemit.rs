//! Emitting binary Intel machine code.

use binemit::{CodeSink, bad_encoding};
use ir::{Function, Inst, InstructionData};
use isa::RegUnit;

include!(concat!(env!("OUT_DIR"), "/binemit-intel.rs"));

pub static RELOC_NAMES: [&'static str; 1] = ["Call"];

// Emit single-byte opcode.
fn put_op1<CS: CodeSink + ?Sized>(bits: u16, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0f00, 0, "Invalid encoding bits for Op1*");
    sink.put1(bits as u8);
}

// Emit two-byte opcode: 0F XX
fn put_op2<CS: CodeSink + ?Sized>(bits: u16, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0f00, 0x0400, "Invalid encoding bits for Op2*");
    sink.put1(0x0f);
    sink.put1(bits as u8);
}

// Mandatory prefix bytes for Mp* opcodes.
const PREFIX: [u8; 3] = [0x66, 0xf3, 0xf2];

// Emit single-byte opcode with mandatory prefix.
fn put_mp1<CS: CodeSink + ?Sized>(bits: u16, sink: &mut CS) {
    debug_assert_eq!(bits & 0x0c00, 0, "Invalid encoding bits for Mp1*");
    let pp = (bits >> 8) & 3;
    sink.put1(PREFIX[(pp - 1) as usize]);
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

fn recipe_op1rr<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Binary { args, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_rr(func.locations[args[0]].unwrap_reg(),
                 func.locations[args[1]].unwrap_reg(),
                 sink);
    } else {
        panic!("Expected Binary format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1rc<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Binary { args, .. } = func.dfg[inst] {
        let bits = func.encodings[inst].bits();
        put_op1(bits, sink);
        modrm_r_bits(func.locations[args[0]].unwrap_reg(), bits, sink);
    } else {
        panic!("Expected Binary format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1rib<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::BinaryImm { arg, imm, .. } = func.dfg[inst] {
        let bits = func.encodings[inst].bits();
        put_op1(bits, sink);
        modrm_r_bits(func.locations[arg].unwrap_reg(), bits, sink);
        let imm: i64 = imm.into();
        sink.put1(imm as u8);
    } else {
        panic!("Expected BinaryImm format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1rid<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::BinaryImm { arg, imm, .. } = func.dfg[inst] {
        let bits = func.encodings[inst].bits();
        put_op1(bits, sink);
        modrm_r_bits(func.locations[arg].unwrap_reg(), bits, sink);
        let imm: i64 = imm.into();
        sink.put4(imm as u32);
    } else {
        panic!("Expected BinaryImm format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1uid<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::UnaryImm { imm, .. } = func.dfg[inst] {
        let bits = func.encodings[inst].bits();
        let reg = func.locations[func.dfg.first_result(inst)].unwrap_reg();
        // The destination register is encoded in the low bits of the opcode. No ModR/M
        put_op1(bits | (reg & 7), sink);
        let imm: i64 = imm.into();
        sink.put4(imm as u32);
    } else {
        panic!("Expected UnaryImm format: {:?}", func.dfg[inst]);
    }
}

// Store recipes.

fn recipe_op1st<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Store { args, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_rm(func.locations[args[1]].unwrap_reg(),
                 func.locations[args[0]].unwrap_reg(),
                 sink);
    } else {
        panic!("Expected Store format: {:?}", func.dfg[inst]);
    }
}

// This is just a tighter register class constraint.
fn recipe_op1st_abcd<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    recipe_op1st(func, inst, sink)
}

fn recipe_mp1st<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Store { args, .. } = func.dfg[inst] {
        put_mp1(func.encodings[inst].bits(), sink);
        modrm_rm(func.locations[args[1]].unwrap_reg(),
                 func.locations[args[0]].unwrap_reg(),
                 sink);
    } else {
        panic!("Expected Store format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1stdisp8<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Store { args, offset, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_disp8(func.locations[args[1]].unwrap_reg(),
                    func.locations[args[0]].unwrap_reg(),
                    sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
    } else {
        panic!("Expected Store format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1stdisp8_abcd<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    recipe_op1stdisp8(func, inst, sink)
}

fn recipe_mp1stdisp8<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Store { args, offset, .. } = func.dfg[inst] {
        put_mp1(func.encodings[inst].bits(), sink);
        modrm_disp8(func.locations[args[1]].unwrap_reg(),
                    func.locations[args[0]].unwrap_reg(),
                    sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
    } else {
        panic!("Expected Store format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1stdisp32<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Store { args, offset, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_disp32(func.locations[args[1]].unwrap_reg(),
                     func.locations[args[0]].unwrap_reg(),
                     sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
    } else {
        panic!("Expected Store format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1stdisp32_abcd<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    recipe_op1stdisp32(func, inst, sink)
}

fn recipe_mp1stdisp32<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Store { args, offset, .. } = func.dfg[inst] {
        put_mp1(func.encodings[inst].bits(), sink);
        modrm_disp32(func.locations[args[1]].unwrap_reg(),
                     func.locations[args[0]].unwrap_reg(),
                     sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
    } else {
        panic!("Expected Store format: {:?}", func.dfg[inst]);
    }
}

// Load recipes

fn recipe_op1ld<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Load { arg, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_rm(func.locations[arg].unwrap_reg(),
                 func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                 sink);
    } else {
        panic!("Expected Load format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1lddisp8<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Load { arg, offset, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_disp8(func.locations[arg].unwrap_reg(),
                    func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                    sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
    } else {
        panic!("Expected Load format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op1lddisp32<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Load { arg, offset, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_disp32(func.locations[arg].unwrap_reg(),
                     func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                     sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
    } else {
        panic!("Expected Load format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op2ld<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Load { arg, .. } = func.dfg[inst] {
        put_op2(func.encodings[inst].bits(), sink);
        modrm_rm(func.locations[arg].unwrap_reg(),
                 func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                 sink);
    } else {
        panic!("Expected Load format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op2lddisp8<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Load { arg, offset, .. } = func.dfg[inst] {
        put_op2(func.encodings[inst].bits(), sink);
        modrm_disp8(func.locations[arg].unwrap_reg(),
                    func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                    sink);
        let offset: i32 = offset.into();
        sink.put1(offset as u8);
    } else {
        panic!("Expected Load format: {:?}", func.dfg[inst]);
    }
}

fn recipe_op2lddisp32<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Load { arg, offset, .. } = func.dfg[inst] {
        put_op2(func.encodings[inst].bits(), sink);
        modrm_disp32(func.locations[arg].unwrap_reg(),
                     func.locations[func.dfg.first_result(inst)].unwrap_reg(),
                     sink);
        let offset: i32 = offset.into();
        sink.put4(offset as u32);
    } else {
        panic!("Expected Load format: {:?}", func.dfg[inst]);
    }
}
