//! Contains the RISC-V instruction encoding logic.
//!
//! These formats are specified in the RISC-V specification in section 2.2.
//! See: https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf
//!
//! Some instructions especially in extensions have slight variations from
//! the base RISC-V specification.

use super::{UImm5, VType};
use crate::isa::riscv64::inst::reg_to_gpr_num;
use crate::Reg;

/// Encode an R-type instruction.
///
/// Layout:
/// 0-------6-7-------11-12------14-15------19-20------24-25-------31
/// | Opcode |   rd     |  funct3  |   rs1    |   rs2    |   funct7  |
pub fn encode_r_type(opcode: u32, rd: Reg, funct3: u32, rs1: Reg, rs2: Reg, funct7: u32) -> u32 {
    let mut bits = 0;
    bits |= opcode & 0b1111111;
    bits |= reg_to_gpr_num(rd) << 7;
    bits |= (funct3 & 0b111) << 12;
    bits |= reg_to_gpr_num(rs1) << 15;
    bits |= reg_to_gpr_num(rs2) << 20;
    bits |= (funct7 & 0b1111111) << 25;
    bits
}

/// Encodes a Vector ALU instruction.
///
/// Fields:
/// - opcode (7 bits)
/// - vd     (5 bits)
/// - funct3 (3 bits)
/// - vs1    (5 bits)
/// - vs2    (5 bits)
/// - vm     (1 bit)
/// - funct6 (6 bits)
///
/// See: https://github.com/riscv/riscv-v-spec/blob/master/valu-format.adoc
pub fn encode_valu(
    opcode: u32,
    vd: Reg,
    funct3: u32,
    vs1: Reg,
    vs2: Reg,
    vm: u32,
    funct6: u32,
) -> u32 {
    let funct6 = funct6 & 0b111111;
    let vm = vm & 0b1;
    let funct7 = (funct6 << 6) | vm;
    encode_r_type(opcode, vd, funct3, vs1, vs2, funct7)
}

/// Encodes a Vector CFG Imm instruction.
///
/// See: https://github.com/riscv/riscv-v-spec/blob/master/vcfg-format.adoc
// TODO: Check if this is any of the known instruction types in the spec.
pub fn encode_vcfg_imm(opcode: u32, rd: Reg, imm: UImm5, vtype: &VType) -> u32 {
    let mut bits = 0;
    bits |= opcode & 0b1111111;
    bits |= reg_to_gpr_num(rd) << 7;
    bits |= 0b111 << 12;
    bits |= (imm.bits() & 0b11111) << 15;
    bits |= (vtype.encode() & 0b1111111111) << 20;
    bits |= 0b11 << 30;
    bits
}
