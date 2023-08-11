//! Contains the RISC-V instruction encoding logic.
//!
//! These formats are specified in the RISC-V specification in section 2.2.
//! See: https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf
//!
//! Some instructions especially in extensions have slight variations from
//! the base RISC-V specification.

use super::{Imm12, Imm5, UImm5, VType};
use crate::isa::riscv64::inst::reg_to_gpr_num;
use crate::isa::riscv64::lower::isle::generated_code::{
    VecAluOpRImm5, VecAluOpRR, VecAluOpRRImm5, VecAluOpRRR, VecAluOpRRRImm5, VecAluOpRRRR,
    VecElementWidth, VecOpCategory, VecOpMasking,
};
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

/// Encode an I-type instruction.
///
/// Layout:
/// 0-------6-7-------11-12------14-15------19-20------------------31
/// | Opcode |   rd     |  width   |   rs1    |     Offset[11:0]    |
pub fn encode_i_type(opcode: u32, rd: WritableReg, width: u32, rs1: Reg, offset: Imm12) -> u32 {
    let mut bits = 0;
    bits |= unsigned_field_width(opcode, 7);
    bits |= reg_to_gpr_num(rd.to_reg()) << 7;
    bits |= unsigned_field_width(width, 3) << 12;
    bits |= reg_to_gpr_num(rs1) << 15;
    bits |= unsigned_field_width(offset.as_u32(), 12) << 20;
    bits
}

/// Encode an S-type instruction.
///
/// Layout:
/// 0-------6-7-------11-12------14-15------19-20---24-25-------------31
/// | Opcode | imm[4:0] |  width   |   base   |  src  |    imm[11:5]   |
pub fn encode_s_type(opcode: u32, width: u32, base: Reg, src: Reg, offset: Imm12) -> u32 {
    let mut bits = 0;
    bits |= unsigned_field_width(opcode, 7);
    bits |= (offset.as_u32() & 0b11111) << 7;
    bits |= unsigned_field_width(width, 3) << 12;
    bits |= reg_to_gpr_num(base) << 15;
    bits |= reg_to_gpr_num(src) << 20;
    bits |= unsigned_field_width(offset.as_u32() >> 5, 7) << 25;
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
    op: VecAluOpRRR,
    vd: WritableReg,
    vs1: Reg,
    vs2: Reg,
    masking: VecOpMasking,
) -> u32 {
    let funct7 = (op.funct6() << 1) | masking.encode();
    encode_r_type_bits(
        op.opcode(),
        reg_to_gpr_num(vd.to_reg()),
        op.funct3(),
        reg_to_gpr_num(vs1),
        reg_to_gpr_num(vs2),
        funct7,
    )
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
pub fn encode_valu_rr_imm(
    op: VecAluOpRRImm5,
    vd: WritableReg,
    imm: Imm5,
    vs2: Reg,
    masking: VecOpMasking,
) -> u32 {
    let funct7 = (op.funct6() << 1) | masking.encode();
    let imm = imm.bits() as u32;
    encode_r_type_bits(
        op.opcode(),
        reg_to_gpr_num(vd.to_reg()),
        op.funct3(),
        imm,
        reg_to_gpr_num(vs2),
        funct7,
    )
}

pub fn encode_valu_rrrr(
    op: VecAluOpRRRR,
    vd: WritableReg,
    vs2: Reg,
    vs1: Reg,
    masking: VecOpMasking,
) -> u32 {
    let funct7 = (op.funct6() << 1) | masking.encode();
    encode_r_type_bits(
        op.opcode(),
        reg_to_gpr_num(vd.to_reg()),
        op.funct3(),
        reg_to_gpr_num(vs1),
        reg_to_gpr_num(vs2),
        funct7,
    )
}

pub fn encode_valu_rrr_imm(
    op: VecAluOpRRRImm5,
    vd: WritableReg,
    imm: Imm5,
    vs2: Reg,
    masking: VecOpMasking,
) -> u32 {
    let funct7 = (op.funct6() << 1) | masking.encode();
    let imm = imm.bits() as u32;
    encode_r_type_bits(
        op.opcode(),
        reg_to_gpr_num(vd.to_reg()),
        op.funct3(),
        imm,
        reg_to_gpr_num(vs2),
        funct7,
    )
}

pub fn encode_valu_rr(op: VecAluOpRR, vd: WritableReg, vs: Reg, masking: VecOpMasking) -> u32 {
    let funct7 = (op.funct6() << 1) | masking.encode();

    let (vs1, vs2) = if op.vs_is_vs2_encoded() {
        (op.aux_encoding(), reg_to_gpr_num(vs))
    } else {
        (reg_to_gpr_num(vs), op.aux_encoding())
    };

    encode_r_type_bits(
        op.opcode(),
        reg_to_gpr_num(vd.to_reg()),
        op.funct3(),
        vs1,
        vs2,
        funct7,
    )
}

pub fn encode_valu_r_imm(
    op: VecAluOpRImm5,
    vd: WritableReg,
    imm: Imm5,
    masking: VecOpMasking,
) -> u32 {
    let funct7 = (op.funct6() << 1) | masking.encode();

    // This is true for this opcode, not sure if there are any other ones.
    debug_assert_eq!(op, VecAluOpRImm5::VmvVI);
    let vs1 = imm.bits() as u32;
    let vs2 = op.aux_encoding();

    encode_r_type_bits(
        op.opcode(),
        reg_to_gpr_num(vd.to_reg()),
        op.funct3(),
        vs1,
        vs2,
        funct7,
    )
}

/// Encodes a Vector CFG Imm instruction.
///
/// See: https://github.com/riscv/riscv-v-spec/blob/master/vcfg-format.adoc
// TODO: Check if this is any of the known instruction types in the spec.
pub fn encode_vcfg_imm(opcode: u32, rd: Reg, imm: UImm5, vtype: &VType) -> u32 {
    let mut bits = 0;
    bits |= unsigned_field_width(opcode, 7);
    bits |= reg_to_gpr_num(rd) << 7;
    bits |= VecOpCategory::OPCFG.encode() << 12;
    bits |= unsigned_field_width(imm.bits(), 5) << 15;
    bits |= unsigned_field_width(vtype.encode(), 10) << 20;
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
    masking: VecOpMasking,
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
    bits |= unsigned_field_width(opcode, 7);
    bits |= reg_to_gpr_num(vd) << 7;
    bits |= width << 12;
    bits |= reg_to_gpr_num(rs1) << 15;
    bits |= unsigned_field_width(lumop, 5) << 20;
    bits |= masking.encode() << 25;
    bits |= unsigned_field_width(mop, 2) << 26;

    // The mew bit (inst[28]) when set is expected to be used to encode expanded
    // memory sizes of 128 bits and above, but these encodings are currently reserved.
    bits |= 0b0 << 28;

    bits |= unsigned_field_width(nf, 3) << 29;
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
    masking: VecOpMasking,
    mop: u32,
    nf: u32,
) -> u32 {
    // This is pretty much the same as the load instruction, just
    // with different names on the fields.
    encode_vmem_load(opcode, vs3, width, rs1, sumop, masking, mop, nf)
}
