//! Contains the RISC-V instruction encoding logic.
//!
//! These formats are specified in the RISC-V specification in section 2.2.
//! See: https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf
//!
//! Some instructions especially in extensions have slight variations from
//! the base RISC-V specification.

use super::{Imm5, UImm5};
use crate::isa::zkasm::inst::reg_to_gpr_num;
use crate::machinst::isle::WritableReg;
use crate::Reg;

fn unsigned_field_width(value: u32, width: u8) -> u32 {
    debug_assert_eq!(value & (!0 << width), 0);
    value
}

/// Layout:
/// 0-------6-7-------11-12------14-15------19-20------24-25-------31
/// | Opcode |   rd     |  funct3  |   rs1    |   rs2    |   funct7  |
fn encode_r_type_bits(opcode: u32, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32) -> u32 {
    let mut bits = 0;
    bits |= unsigned_field_width(opcode, 7);
    bits |= unsigned_field_width(rd, 5) << 7;
    bits |= unsigned_field_width(funct3, 3) << 12;
    bits |= unsigned_field_width(rs1, 5) << 15;
    bits |= unsigned_field_width(rs2, 5) << 20;
    bits |= unsigned_field_width(funct7, 7) << 25;
    bits
}

/// Encode an R-type instruction.
pub fn encode_r_type(
    opcode: u32,
    rd: WritableReg,
    funct3: u32,
    rs1: Reg,
    rs2: Reg,
    funct7: u32,
) -> u32 {
    encode_r_type_bits(
        opcode,
        reg_to_gpr_num(rd.to_reg()),
        funct3,
        reg_to_gpr_num(rs1),
        reg_to_gpr_num(rs2),
        funct7,
    )
}
