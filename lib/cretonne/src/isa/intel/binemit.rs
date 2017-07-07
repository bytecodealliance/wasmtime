//! Emitting binary Intel machine code.

use binemit::{CodeSink, Reloc, bad_encoding};
use ir::{self, Function, Inst, InstructionData, MemFlags};
use ir::immediates::{Imm64, Offset32};
use isa::RegUnit;

include!(concat!(env!("OUT_DIR"), "/binemit-intel.rs"));

/// Intel relocations.
pub enum RelocKind {
    /// A 4-byte relative function reference. Based from relocation + 4 bytes.
    PCRel4,
}

pub static RELOC_NAMES: [&'static str; 1] = ["PCRel4"];

impl Into<Reloc> for RelocKind {
    fn into(self) -> Reloc {
        Reloc(self as u16)
    }
}

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

fn recipe_op1rr<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit,
                                       in_reg1: RegUnit) {
    put_op1(bits, sink);
    modrm_rr(in_reg0, in_reg1, sink);
}

fn recipe_op1ur<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit,
                                       out_reg0: RegUnit) {
    put_op1(bits, sink);
    modrm_rr(out_reg0, in_reg0, sink);
}

fn recipe_op1rc<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit) {
    put_op1(bits, sink);
    modrm_r_bits(in_reg0, bits, sink);
}

fn recipe_op1rib<CS: CodeSink + ?Sized>(_func: &Function,
                                        _inst: Inst,
                                        sink: &mut CS,
                                        bits: u16,
                                        in_reg0: RegUnit,
                                        imm: Imm64) {
    put_op1(bits, sink);
    modrm_r_bits(in_reg0, bits, sink);
    let imm: i64 = imm.into();
    sink.put1(imm as u8);
}

fn recipe_op1rid<CS: CodeSink + ?Sized>(_func: &Function,
                                        _inst: Inst,
                                        sink: &mut CS,
                                        bits: u16,
                                        in_reg0: RegUnit,
                                        imm: Imm64) {
    put_op1(bits, sink);
    modrm_r_bits(in_reg0, bits, sink);
    let imm: i64 = imm.into();
    sink.put4(imm as u32);
}

fn recipe_op1uid<CS: CodeSink + ?Sized>(_func: &Function,
                                        _inst: Inst,
                                        sink: &mut CS,
                                        bits: u16,
                                        imm: Imm64,
                                        out_reg0: RegUnit) {
    // The destination register is encoded in the low bits of the opcode. No ModR/M
    put_op1(bits | (out_reg0 & 7), sink);
    let imm: i64 = imm.into();
    sink.put4(imm as u32);
}

// Store recipes.

fn recipe_op1st<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit,
                                       in_reg1: RegUnit,
                                       _flags: MemFlags,
                                       _offset: Offset32) {
    put_op1(bits, sink);
    modrm_rm(in_reg1, in_reg0, sink);
}

// This is just a tighter register class constraint.
fn recipe_op1st_abcd<CS: CodeSink + ?Sized>(func: &Function,
                                            inst: Inst,
                                            sink: &mut CS,
                                            bits: u16,
                                            in_reg0: RegUnit,
                                            in_reg1: RegUnit,
                                            flags: MemFlags,
                                            offset: Offset32) {
    recipe_op1st(func, inst, sink, bits, in_reg0, in_reg1, flags, offset)
}

fn recipe_mp1st<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit,
                                       in_reg1: RegUnit,
                                       _flags: MemFlags,
                                       _offset: Offset32) {
    put_mp1(bits, sink);
    modrm_rm(in_reg1, in_reg0, sink);
}

fn recipe_op1stdisp8<CS: CodeSink + ?Sized>(_func: &Function,
                                            _inst: Inst,
                                            sink: &mut CS,
                                            bits: u16,
                                            in_reg0: RegUnit,
                                            in_reg1: RegUnit,
                                            _flags: MemFlags,
                                            offset: Offset32) {
    put_op1(bits, sink);
    modrm_disp8(in_reg1, in_reg0, sink);
    let offset: i32 = offset.into();
    sink.put1(offset as u8);
}

fn recipe_op1stdisp8_abcd<CS: CodeSink + ?Sized>(func: &Function,
                                                 inst: Inst,
                                                 sink: &mut CS,
                                                 bits: u16,
                                                 in_reg0: RegUnit,
                                                 in_reg1: RegUnit,
                                                 flags: MemFlags,
                                                 offset: Offset32) {
    recipe_op1stdisp8(func, inst, sink, bits, in_reg0, in_reg1, flags, offset)
}

fn recipe_mp1stdisp8<CS: CodeSink + ?Sized>(_func: &Function,
                                            _inst: Inst,
                                            sink: &mut CS,
                                            bits: u16,
                                            in_reg0: RegUnit,
                                            in_reg1: RegUnit,
                                            _flags: MemFlags,
                                            offset: Offset32) {
    put_mp1(bits, sink);
    modrm_disp8(in_reg1, in_reg0, sink);
    let offset: i32 = offset.into();
    sink.put1(offset as u8);
}

fn recipe_op1stdisp32<CS: CodeSink + ?Sized>(_func: &Function,
                                             _inst: Inst,
                                             sink: &mut CS,
                                             bits: u16,
                                             in_reg0: RegUnit,
                                             in_reg1: RegUnit,
                                             _flags: MemFlags,
                                             offset: Offset32) {
    put_op1(bits, sink);
    modrm_disp32(in_reg1, in_reg0, sink);
    let offset: i32 = offset.into();
    sink.put4(offset as u32);
}

fn recipe_op1stdisp32_abcd<CS: CodeSink + ?Sized>(func: &Function,
                                                  inst: Inst,
                                                  sink: &mut CS,
                                                  bits: u16,
                                                  in_reg0: RegUnit,
                                                  in_reg1: RegUnit,
                                                  flags: MemFlags,
                                                  offset: Offset32) {
    recipe_op1stdisp32(func, inst, sink, bits, in_reg0, in_reg1, flags, offset)
}

fn recipe_mp1stdisp32<CS: CodeSink + ?Sized>(_func: &Function,
                                             _inst: Inst,
                                             sink: &mut CS,
                                             bits: u16,
                                             in_reg0: RegUnit,
                                             in_reg1: RegUnit,
                                             _flags: MemFlags,
                                             offset: Offset32) {
    put_mp1(bits, sink);
    modrm_disp32(in_reg1, in_reg0, sink);
    let offset: i32 = offset.into();
    sink.put4(offset as u32);
}

// Load recipes

fn recipe_op1ld<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit,
                                       _flags: MemFlags,
                                       _offset: Offset32,
                                       out_reg0: RegUnit) {
    put_op1(bits, sink);
    modrm_rm(in_reg0, out_reg0, sink);
}

fn recipe_op1lddisp8<CS: CodeSink + ?Sized>(_func: &Function,
                                            _inst: Inst,
                                            sink: &mut CS,
                                            bits: u16,
                                            in_reg0: RegUnit,
                                            _flags: MemFlags,
                                            offset: Offset32,
                                            out_reg0: RegUnit) {
    put_op1(bits, sink);
    modrm_disp8(in_reg0, out_reg0, sink);
    let offset: i32 = offset.into();
    sink.put1(offset as u8);
}

fn recipe_op1lddisp32<CS: CodeSink + ?Sized>(_func: &Function,
                                             _inst: Inst,
                                             sink: &mut CS,
                                             bits: u16,
                                             in_reg0: RegUnit,
                                             _flags: MemFlags,
                                             offset: Offset32,
                                             out_reg0: RegUnit) {
    put_op1(bits, sink);
    modrm_disp32(in_reg0, out_reg0, sink);
    let offset: i32 = offset.into();
    sink.put4(offset as u32);
}

fn recipe_op2ld<CS: CodeSink + ?Sized>(_func: &Function,
                                       _inst: Inst,
                                       sink: &mut CS,
                                       bits: u16,
                                       in_reg0: RegUnit,
                                       _flags: MemFlags,
                                       _offset: Offset32,
                                       out_reg0: RegUnit) {
    put_op2(bits, sink);
    modrm_rm(in_reg0, out_reg0, sink);
}

fn recipe_op2lddisp8<CS: CodeSink + ?Sized>(_func: &Function,
                                            _inst: Inst,
                                            sink: &mut CS,
                                            bits: u16,
                                            in_reg0: RegUnit,
                                            _flags: MemFlags,
                                            offset: Offset32,
                                            out_reg0: RegUnit) {
    put_op2(bits, sink);
    modrm_disp8(in_reg0, out_reg0, sink);
    let offset: i32 = offset.into();
    sink.put1(offset as u8);
}

fn recipe_op2lddisp32<CS: CodeSink + ?Sized>(_func: &Function,
                                             _inst: Inst,
                                             sink: &mut CS,
                                             bits: u16,
                                             in_reg0: RegUnit,
                                             _flags: MemFlags,
                                             offset: Offset32,
                                             out_reg0: RegUnit) {
    put_op2(bits, sink);
    modrm_disp32(in_reg0, out_reg0, sink);
    let offset: i32 = offset.into();
    sink.put4(offset as u32);
}

fn recipe_op1call_id<CS: CodeSink + ?Sized>(_func: &Function,
                                            _inst: Inst,
                                            sink: &mut CS,
                                            bits: u16,
                                            func_ref: ir::FuncRef) {
    put_op1(bits, sink);
    sink.reloc_func(RelocKind::PCRel4.into(), func_ref);
    sink.put4(0);
}

fn recipe_op1call_r<CS: CodeSink + ?Sized>(_func: &Function,
                                           _inst: Inst,
                                           sink: &mut CS,
                                           bits: u16,
                                           in_reg0: RegUnit,
                                           _sig_ref: ir::SigRef) {
    put_op1(bits, sink);
    modrm_r_bits(in_reg0, bits, sink);
}

fn recipe_op1ret<CS: CodeSink + ?Sized>(_func: &Function, _inst: Inst, sink: &mut CS, bits: u16) {
    put_op1(bits, sink);
}
