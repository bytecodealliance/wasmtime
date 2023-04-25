//! Contains the RISC-V instruction encoding logic.
//!
//! These formats are specified in the RISC-V specification in section 2.2.
//! See: https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf
//!
//! Some instructions especially in extensions have slight variations from
//! the base RISC-V specification.

use super::{Imm5, UImm5, VType};
use crate::isa::riscv64::inst::reg_to_gpr_num;
use crate::isa::riscv64::lower::isle::generated_code::VecElementWidth;
use crate::Reg;

/// Layout:
/// 0-------6-7-------11-12------14-15------19-20------24-25-------31
/// | Opcode |   rd     |  funct3  |   rs1    |   rs2    |   funct7  |
fn encode_r_type_bits(opcode: u32, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32) -> u32 {
    let mut bits = 0;
    bits |= opcode & 0b1111111;
    bits |= (rd & 0b11111) << 7;
    bits |= (funct3 & 0b111) << 12;
    bits |= (rs1 & 0b11111) << 15;
    bits |= (rs2 & 0b11111) << 20;
    bits |= (funct7 & 0b1111111) << 25;
    bits
}

/// Encode an R-type instruction.
pub fn encode_r_type(opcode: u32, rd: Reg, funct3: u32, rs1: Reg, rs2: Reg, funct7: u32) -> u32 {
    encode_r_type_bits(
        opcode,
        reg_to_gpr_num(rd),
        funct3,
        reg_to_gpr_num(rs1),
        reg_to_gpr_num(rs2),
        funct7,
    )
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
    // VALU is just VALUImm with the register in the immediate field.
    let imm = Imm5::maybe_from_i8((reg_to_gpr_num(vs1) as i8) << 3 >> 3).unwrap();
    encode_valu_imm(opcode, vd, funct3, imm, vs2, vm, funct6)
}

/// Encodes a Vector ALU+Imm instruction.
/// This is just a Vector ALU instruction with an immediate in the VS1 field.
///
/// Fields:
/// - opcode (7 bits)
/// - vd     (5 bits)
/// - funct3 (3 bits)
/// - imm    (5 bits)
/// - vs2    (5 bits)
/// - vm     (1 bit)
/// - funct6 (6 bits)
///
/// See: https://github.com/riscv/riscv-v-spec/blob/master/valu-format.adoc
pub fn encode_valu_imm(
    opcode: u32,
    vd: Reg,
    funct3: u32,
    imm: Imm5,
    vs2: Reg,
    vm: u32,
    funct6: u32,
) -> u32 {
    let funct6 = funct6 & 0b111111;
    let vm = vm & 0b1;
    let funct7 = (funct6 << 1) | vm;
    let imm = (imm.bits() & 0b11111) as u32;
    encode_r_type_bits(
        opcode,
        reg_to_gpr_num(vd),
        funct3,
        imm,
        reg_to_gpr_num(vs2),
        funct7,
    )
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

/// Encodes a Vector Mem Unit Stride Load instruction.
///
/// See: https://github.com/riscv/riscv-v-spec/blob/master/vmem-format.adoc
/// TODO: These instructions share opcode space with LOAD-FP and STORE-FP
pub fn encode_vmem_load(
    opcode: u32,
    vd: Reg,
    width: VecElementWidth,
    rs1: Reg,
    lumop: u32,
    vm: u32,
    mop: u32,
    nf: u32,
) -> u32 {
    // Width is encoded differently to avoid a clash with the FP load/store sizes.
    let width = match width {
        VecElementWidth::E8 => 0b000,
        VecElementWidth::E16 => 0b101,
        VecElementWidth::E32 => 0b110,
        VecElementWidth::E64 => 0b111,
    };

    let mut bits = 0;
    bits |= opcode & 0b1111111;
    bits |= reg_to_gpr_num(vd) << 7;
    bits |= width << 12;
    bits |= reg_to_gpr_num(rs1) << 15;
    bits |= (lumop & 0b11111) << 20;
    bits |= (vm & 0b1) << 25;
    bits |= (mop & 0b11) << 26;

    // The mew bit (inst[28]) when set is expected to be used to encode expanded
    // memory sizes of 128 bits and above, but these encodings are currently reserved.
    bits |= 0b0 << 28;

    bits |= (nf & 0b111) << 29;
    bits
}

/// Encodes a Vector Mem Unit Stride Load instruction.
///
/// See: https://github.com/riscv/riscv-v-spec/blob/master/vmem-format.adoc
/// TODO: These instructions share opcode space with LOAD-FP and STORE-FP
pub fn encode_vmem_store(
    opcode: u32,
    vs3: Reg,
    width: VecElementWidth,
    rs1: Reg,
    sumop: u32,
    vm: u32,
    mop: u32,
    nf: u32,
) -> u32 {
    // This is pretty much the same as the load instruction, just
    // with different names on the fields.
    encode_vmem_load(opcode, vs3, width, rs1, sumop, vm, mop, nf)
}
