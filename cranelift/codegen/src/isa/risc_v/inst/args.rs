//! AArch64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::ir::condcodes::{CondCode, FloatCC};

use super::*;

pub static WORD_SIZE: u8 = 8;

static rm: u32 = 0;

/// An addressing mode specified for a load/store operation.
#[derive(Clone, Debug, Copy)]
pub enum AMode {
    /// Arbitrary offset from a register. Converted to generation of large
    /// offsets with multiple instructions as necessary during code emission.
    RegOffset(Reg, i64, Type),
    /// Offset from the stack pointer.
    SPOffset(i64, Type),

    /// Offset from the frame pointer.
    FPOffset(i64, Type),

    /// Offset from the "nominal stack pointer", which is where the real SP is
    /// just after stack and spill slots are allocated in the function prologue.
    /// At emission time, this is converted to `SPOffset` with a fixup added to
    /// the offset constant. The fixup is a running value that is tracked as
    /// emission iterates through instructions in linear order, and can be
    /// adjusted up and down with [Inst::VirtualSPOffsetAdj].
    ///
    /// The standard ABI is in charge of handling this (by emitting the
    /// adjustment meta-instructions). It maintains the invariant that "nominal
    /// SP" is where the actual SP is after the function prologue and before
    /// clobber pushes. See the diagram in the documentation for
    /// [crate::isa::aarch64::abi](the ABI module) for more details.
    NominalSPOffset(i64, Type),
}

impl AMode {
    pub(crate) fn reg_offset(reg: Reg, imm: i64, Type: Type) -> AMode {
        AMode::RegOffset(reg, imm, Type)
    }

    pub(crate) fn get_base_register(&self) -> Reg {
        match self {
            &AMode::RegOffset(reg, ..) => reg,
            &AMode::SPOffset(..) => stack_reg(),
            &AMode::FPOffset(..) => fp_reg(),
            &AMode::NominalSPOffset(..) => stack_reg(),
        }
    }

    pub(crate) fn get_offset(&self) -> i64 {
        match self {
            &AMode::RegOffset(_, offset, ..) => offset,
            &AMode::SPOffset(offset, _) => offset,
            &AMode::FPOffset(offset, _) => offset,
            &AMode::NominalSPOffset(offset, _) => offset,
        }
    }
}

impl Into<AMode> for StackAMode {
    fn into(self) -> AMode {
        match self {
            StackAMode::FPOffset(offset, ty) => AMode::FPOffset(offset, ty),
            StackAMode::SPOffset(offset, ty) => AMode::SPOffset(offset, ty),
            StackAMode::NominalSPOffset(offset, ty) => AMode::NominalSPOffset(offset, ty),
        }
    }
}

/// risc-v always take two register to compar
/// brz can be compare with zero register which has the value 0
#[derive(Clone, Copy, Debug)]
pub struct CondBrKind {
    pub kind: IntCC,
    pub rs1: Reg,
    pub rs2: Reg,
}

impl CondBrKind {
    pub(crate) fn op_code(self) -> u32 {
        0b1100011
    }

    /*
       funct3 and if need inverse the register
    */
    pub(crate) fn funct3(&self) -> (u32, bool) {
        match self.kind {
            IntCC::Equal => (0b000, false),
            IntCC::NotEqual => (0b001, false),
            IntCC::SignedLessThan => (0b100, false),
            IntCC::SignedGreaterThanOrEqual => (0b101, false),

            IntCC::SignedGreaterThan => (0b100, true),
            IntCC::SignedLessThanOrEqual => (0b101, true),

            IntCC::UnsignedLessThan => (0b110, false),
            IntCC::UnsignedGreaterThanOrEqual => (0b111, false),

            IntCC::UnsignedGreaterThan => (0b110, true),
            IntCC::UnsignedLessThanOrEqual => (0b111, true),
            IntCC::Overflow => todo!(),
            IntCC::NotOverflow => todo!(),
        }
    }

    pub(crate) fn kind_name(&self) -> String {
        format!("b{}", self.kind.to_static_str())
    }

    pub(crate) fn emit(self) -> u32 {
        let (funct3, inverse_register) = self.funct3();
        let (rs1, rs2) = if !inverse_register {
            (self.rs1, self.rs2)
        } else {
            (self.rs2, self.rs1)
        };
        self.op_code()
            | funct3 << 12
            | (rs1.get_hw_encoding() as u32) << 15
            | (rs2.get_hw_encoding() as u32) << 20
    }

    pub(crate) fn inverse(self) -> Self {
        Self {
            kind: self.kind.inverse(),
            ..self
        }
    }
}

/*
    Op are always defined in isle file and implemented Dispaly.
    I use this to generate op_name instead of boring work.
*/
fn get_op_name<T: std::fmt::Debug>(op: T) -> String {
    format!("{:?}", op).to_lowercase().replace("_", ".")
}

impl AluOPRRRR {
    pub(crate) fn op_name(self) -> String {
        get_op_name(self)
    }

    pub(crate) fn funct2(self) -> u32 {
        match self {
            AluOPRRRR::FMADD_S | AluOPRRRR::FMSUB_S | AluOPRRRR::FNMSUB_S | AluOPRRRR::FNMADD_S => {
                0
            }
            AluOPRRRR::FMADD_D | AluOPRRRR::FMSUB_D | AluOPRRRR::FNMSUB_D | AluOPRRRR::FNMADD_D => {
                1
            }
        }
    }

    pub(crate) fn funct3(self) -> u32 {
        //todo look like all undefined, all zero
        0
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            AluOPRRRR::FMADD_S => 0b1000011,
            AluOPRRRR::FMSUB_S => 0b1000111,
            AluOPRRRR::FNMSUB_S => 0b1001011,
            AluOPRRRR::FNMADD_S => 0b1001111,
            AluOPRRRR::FMADD_D => 0b1000011,
            AluOPRRRR::FMSUB_D => 0b1000111,
            AluOPRRRR::FNMSUB_D => 0b1001011,
            AluOPRRRR::FNMADD_D => 0b1001111,
        }
    }
}

impl AluOPRR {
    pub(crate) fn op_name(self) -> String {
        get_op_name(self)
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            AluOPRR::FSQRT_S
            | AluOPRR::FCVT_W_S
            | AluOPRR::FCVT_WU_S
            | AluOPRR::FMV_X_W
            | AluOPRR::FCLASS_S
            | AluOPRR::FCVT_S_W
            | AluOPRR::FCVT_S_WU
            | AluOPRR::FMV_W_X => 0b1010011,

            AluOPRR::FCVT_L_S | AluOPRR::FCVT_LU_S | AluOPRR::FCVT_S_L | AluOPRR::FCVT_S_LU => {
                0b1010011
            }

            AluOPRR::FCVT_L_D
            | AluOPRR::FCVT_LU_D
            | AluOPRR::FMV_X_D
            | AluOPRR::FCVT_D_L
            | AluOPRR::FCVT_D_LU
            | AluOPRR::FMV_D_X => 0b1010011,

            AluOPRR::FSQRT_D
            | AluOPRR::FCVT_S_D
            | AluOPRR::FCVT_D_S
            | AluOPRR::FCLASS_D
            | AluOPRR::FCVT_W_D
            | AluOPRR::FCVT_WU_D
            | AluOPRR::FCVT_D_W
            | AluOPRR::FCVT_D_WU => 0b1010011,
        }
    }
    /*
    todo in rs2 position.
    What should I call this.
        */
    pub(crate) fn rs2(self) -> u32 {
        match self {
            AluOPRR::FSQRT_S => 0b00000,
            AluOPRR::FCVT_W_S => 0b00000,
            AluOPRR::FCVT_WU_S => 0b00001,
            AluOPRR::FMV_X_W => 0b00000,
            AluOPRR::FCLASS_S => 0b00000,
            AluOPRR::FCVT_S_W => 0b00000,
            AluOPRR::FCVT_S_WU => 0b00001,
            AluOPRR::FMV_W_X => 0b00000,
            AluOPRR::FCVT_L_S => 0b00010,
            AluOPRR::FCVT_LU_S => 0b00011,
            AluOPRR::FCVT_S_L => 0b00010,
            AluOPRR::FCVT_S_LU => 0b00011,
            AluOPRR::FCVT_L_D => 0b00010,
            AluOPRR::FCVT_LU_D => 0b00011,
            AluOPRR::FMV_X_D => 0b00000,
            AluOPRR::FCVT_D_L => 0b00010,
            AluOPRR::FCVT_D_LU => 0b00011,
            AluOPRR::FMV_D_X => 0b00000,
            AluOPRR::FCVT_S_D => 0b00001,
            AluOPRR::FCVT_D_S => 0b00000,
            AluOPRR::FCLASS_D => 0b00000,
            AluOPRR::FCVT_W_D => 0b00000,
            AluOPRR::FCVT_WU_D => 0b00001,
            AluOPRR::FCVT_D_W => 0b00000,
            AluOPRR::FCVT_D_WU => 0b00001,
            AluOPRR::FSQRT_D => 0b00000,
        }
    }
    pub(crate) fn funct7(self) -> u32 {
        match self {
            AluOPRR::FSQRT_S => 0b0101100,
            AluOPRR::FCVT_W_S => 0b1100000,
            AluOPRR::FCVT_WU_S => 0b1100000,
            AluOPRR::FMV_X_W => 0b1110000,
            AluOPRR::FCLASS_S => 0b1110000,
            AluOPRR::FCVT_S_W => 0b1101000,
            AluOPRR::FCVT_S_WU => 0b1101000,
            AluOPRR::FMV_W_X => 0b1111000,
            AluOPRR::FCVT_L_S => 0b1100000,
            AluOPRR::FCVT_LU_S => 0b1100000,
            AluOPRR::FCVT_S_L => 0b1101000,
            AluOPRR::FCVT_S_LU => 0b1101000,
            AluOPRR::FCVT_L_D => 0b1100001,
            AluOPRR::FCVT_LU_D => 0b1100001,
            AluOPRR::FMV_X_D => 0b1110001,
            AluOPRR::FCVT_D_L => 0b1101001,
            AluOPRR::FCVT_D_LU => 0b1101001,
            AluOPRR::FMV_D_X => 0b1111001,
            AluOPRR::FCVT_S_D => 0b0100000,
            AluOPRR::FCVT_D_S => 0b0100001,
            AluOPRR::FCLASS_D => 0b1110001,
            AluOPRR::FCVT_W_D => 0b1100001,
            AluOPRR::FCVT_WU_D => 0b1100001,
            AluOPRR::FCVT_D_W => 0b1101001,
            AluOPRR::FCVT_D_WU => 0b1101001,
            AluOPRR::FSQRT_D => 0b0101101,
        }
    }

    pub(crate) fn funct3(self) -> u32 {
        match self {
            AluOPRR::FSQRT_S => rm,
            AluOPRR::FCVT_W_S => rm,
            AluOPRR::FCVT_WU_S => rm,
            AluOPRR::FMV_X_W => 0b000,
            AluOPRR::FCLASS_S => 0b001,
            AluOPRR::FCVT_S_W => rm,
            AluOPRR::FCVT_S_WU => rm,
            AluOPRR::FMV_W_X => 0b000,

            AluOPRR::FCVT_L_S => rm,
            AluOPRR::FCVT_LU_S => rm,
            AluOPRR::FCVT_S_L => rm,
            AluOPRR::FCVT_S_LU => rm,

            AluOPRR::FCVT_L_D => rm,
            AluOPRR::FCVT_LU_D => rm,
            AluOPRR::FMV_X_D => 0b000,
            AluOPRR::FCVT_D_L => rm,
            AluOPRR::FCVT_D_LU => rm,
            AluOPRR::FMV_D_X => 0b000,
            AluOPRR::FCVT_S_D => rm,
            AluOPRR::FCVT_D_S => rm,
            AluOPRR::FCLASS_D => 0b001,
            AluOPRR::FCVT_W_D => rm,
            AluOPRR::FCVT_WU_D => rm,
            AluOPRR::FCVT_D_W => rm,
            AluOPRR::FCVT_D_WU => rm,
            AluOPRR::FSQRT_D => rm,
        }
    }
}

impl AluOPRRR {
    pub fn op_name(self) -> String {
        get_op_name(self)
    }

    pub fn funct3(self) -> u32 {
        match self {
            AluOPRRR::ADD => 0b000,
            AluOPRRR::SLL => 0b001,
            AluOPRRR::SLT => 0b010,
            AluOPRRR::SLTU => 0b011,
            AluOPRRR::XOR => 0b100,
            AluOPRRR::SRL => 0b101,
            AluOPRRR::SRA => 0b101,
            AluOPRRR::OR => 0b110,
            AluOPRRR::AND => 0b111,
            AluOPRRR::SUB => 0b000,

            AluOPRRR::ADDW => 0b000,
            AluOPRRR::SUBW => 0b000,
            AluOPRRR::SLLW => 0b001,
            AluOPRRR::SRLW => 0b101,
            AluOPRRR::Sraw => 0b101,

            AluOPRRR::MUL => 0b000,
            AluOPRRR::MULH => 0b001,
            AluOPRRR::MULHSU => 0b010,
            AluOPRRR::MULHU => 0b011,
            AluOPRRR::DIV => 0b100,
            AluOPRRR::DIVU => 0b101,
            AluOPRRR::REM => 0b110,
            AluOPRRR::REMU => 0b111,

            AluOPRRR::MULW => 0b000,
            AluOPRRR::DIVW => 0b100,
            AluOPRRR::DIVUW => 0b101,
            AluOPRRR::REMW => 0b110,
            AluOPRRR::REMUW => 0b111,

            AluOPRRR::FADD_S => rm,
            AluOPRRR::FSUB_S => rm,
            AluOPRRR::FMUL_S => rm,
            AluOPRRR::FDIV_S => rm,

            AluOPRRR::FSGNJ_S => 0b000,
            AluOPRRR::FSGNJN_S => 0b001,
            AluOPRRR::FSGNJX_S => 0b010,
            AluOPRRR::FMIN_S => 0b000,
            AluOPRRR::FMAX_S => 0b001,

            AluOPRRR::FEQ_S => 0b010,
            AluOPRRR::FLT_S => 0b001,
            AluOPRRR::FLE_S => 0b000,

            AluOPRRR::FADD_D => rm,
            AluOPRRR::FSUB_D => rm,
            AluOPRRR::FMUL_D => rm,
            AluOPRRR::FDIV_D => rm,

            AluOPRRR::FSGNJ_D => 0b000,
            AluOPRRR::FSGNJN_D => 0b001,
            AluOPRRR::FSGNJX_D => 0b010,
            AluOPRRR::FMIN_D => 0b000,
            AluOPRRR::FMAX_D => 0b001,
            AluOPRRR::FEQ_D => 0b010,
            AluOPRRR::FLT_D => 0b001,
            AluOPRRR::FLE_D => 0b001,
        }
    }

    pub fn op_code(self) -> u32 {
        match self {
            AluOPRRR::ADD
            | AluOPRRR::SUB
            | AluOPRRR::SLL
            | AluOPRRR::SLT
            | AluOPRRR::SLTU
            | AluOPRRR::XOR
            | AluOPRRR::SRL
            | AluOPRRR::SRA
            | AluOPRRR::OR
            | AluOPRRR::AND => 0b0110011,

            AluOPRRR::ADDW | AluOPRRR::SUBW | AluOPRRR::SLLW | AluOPRRR::SRLW | AluOPRRR::Sraw => {
                0b0111011
            }

            AluOPRRR::MUL
            | AluOPRRR::MULH
            | AluOPRRR::MULHSU
            | AluOPRRR::MULHU
            | AluOPRRR::DIV
            | AluOPRRR::DIVU
            | AluOPRRR::REM
            | AluOPRRR::REMU => 0b0110011,

            AluOPRRR::MULW
            | AluOPRRR::DIVW
            | AluOPRRR::DIVUW
            | AluOPRRR::REMW
            | AluOPRRR::REMUW => 0b0111011,

            AluOPRRR::FADD_S
            | AluOPRRR::FSUB_S
            | AluOPRRR::FMUL_S
            | AluOPRRR::FDIV_S
            | AluOPRRR::FSGNJ_S
            | AluOPRRR::FSGNJN_S
            | AluOPRRR::FSGNJX_S
            | AluOPRRR::FMIN_S
            | AluOPRRR::FMAX_S
            | AluOPRRR::FEQ_S
            | AluOPRRR::FLT_S
            | AluOPRRR::FLE_S => 0b1010011,

            AluOPRRR::FADD_D
            | AluOPRRR::FSUB_D
            | AluOPRRR::FMUL_D
            | AluOPRRR::FDIV_D
            | AluOPRRR::FSGNJ_D
            | AluOPRRR::FSGNJN_D
            | AluOPRRR::FSGNJX_D
            | AluOPRRR::FMIN_D
            | AluOPRRR::FMAX_D
            | AluOPRRR::FEQ_D
            | AluOPRRR::FLT_D
            | AluOPRRR::FLE_D => 0b1010011,
        }
    }

    pub fn funct7(self) -> u32 {
        match self {
            AluOPRRR::ADD => 0b0000000,
            AluOPRRR::SUB => 0b0100000,
            AluOPRRR::SLL => 0b0000000,
            AluOPRRR::SLT => 0b0000000,
            AluOPRRR::SLTU => 0b0000000,
            AluOPRRR::XOR => 0b0000000,
            AluOPRRR::SRL => 0b0000000,
            AluOPRRR::SRA => 0b0100000,
            AluOPRRR::OR => 0b0000000,
            AluOPRRR::AND => 0b0000000,

            AluOPRRR::ADDW => 0b0000000,
            AluOPRRR::SUBW => 0b0100000,
            AluOPRRR::SLLW => 0b0000000,
            AluOPRRR::SRLW => 0b0000000,
            AluOPRRR::Sraw => 0b0100000,

            AluOPRRR::MUL => 0b0000001,
            AluOPRRR::MULH => 0b0000001,
            AluOPRRR::MULHSU => 0b0000001,
            AluOPRRR::MULHU => 0b0000001,
            AluOPRRR::DIV => 0b0000001,
            AluOPRRR::DIVU => 0b0000001,
            AluOPRRR::REM => 0b0000001,
            AluOPRRR::REMU => 0b0000001,

            AluOPRRR::MULW => 0b0000001,
            AluOPRRR::DIVW => 0b0000001,
            AluOPRRR::DIVUW => 0b0000001,
            AluOPRRR::REMW => 0b0000001,
            AluOPRRR::REMUW => 0b0000001,

            AluOPRRR::FADD_S => 0b0000000,
            AluOPRRR::FSUB_S => 0b0000100,
            AluOPRRR::FMUL_S => 0b0001000,
            AluOPRRR::FDIV_S => 0b0001100,

            AluOPRRR::FSGNJ_S => 0b0010000,
            AluOPRRR::FSGNJN_S => 0b0010000,
            AluOPRRR::FSGNJX_S => 0b0010000,
            AluOPRRR::FMIN_S => 0b0010100,
            AluOPRRR::FMAX_S => 0b0010100,
            AluOPRRR::FEQ_S => 0b1010000,
            AluOPRRR::FLT_S => 0b1010000,
            AluOPRRR::FLE_S => 0b1010000,
            AluOPRRR::FADD_D => 0b0000001,
            AluOPRRR::FSUB_D => 0b0000101,
            AluOPRRR::FMUL_D => 0b0001001,
            AluOPRRR::FDIV_D => 0b0001101,

            AluOPRRR::FSGNJ_D => 0b0010001,
            AluOPRRR::FSGNJN_D => 0b0010001,
            AluOPRRR::FSGNJX_D => 0b0010001,
            AluOPRRR::FMIN_D => 0b0010101,
            AluOPRRR::FMAX_D => 0b0010101,
            AluOPRRR::FEQ_D => 0b1010001,
            AluOPRRR::FLT_D => 0b1010001,
            AluOPRRR::FLE_D => 0b1010001,
        }
    }
}

impl AluOPRRI {
    /*
        int 64bit this is 6 bit length, other is 7 bit length
    */
    pub(crate) fn option_funct6(self) -> Option<u32> {
        match self {
            AluOPRRI::SLLI => Some(0b00_0000),
            AluOPRRI::SRLI => Some(0b00_0000),
            AluOPRRI::SRAI => Some(0b01_0000),
            _ => None,
        }
    }
    /*
        SLLIW .. operation on 32-bit value , only need 5-bite shift size.
    */
    pub(crate) fn option_funct7(self) -> Option<u32> {
        match self {
            Self::SLLIW => Some(0b0000000),
            Self::SRLIW => Some(0b0000000),
            Self::SRAIW => Some(0100000),
            _ => None,
        }
    }

    pub(crate) fn op_name(self) -> String {
        get_op_name(self)
    }

    pub fn funct3(self) -> u32 {
        match self {
            AluOPRRI::ADDI => 0b000,
            AluOPRRI::SLTI => 0b010,
            AluOPRRI::SLTIU => 0b011,
            AluOPRRI::XORI => 0b100,
            AluOPRRI::ORI => 0b110,
            AluOPRRI::ANDI => 0b111,
            AluOPRRI::SLLI => 0b001,
            AluOPRRI::SRLI => 0b101,
            AluOPRRI::SRAI => 0b101,
            AluOPRRI::ADDIW => 0b000,
            AluOPRRI::SLLIW => 0b001,
            AluOPRRI::SRLIW => 0b101,
            AluOPRRI::SRAIW => 0b101,
        }
    }

    pub fn op_code(self) -> u32 {
        match self {
            AluOPRRI::ADDI
            | AluOPRRI::SLTI
            | AluOPRRI::SLTIU
            | AluOPRRI::XORI
            | AluOPRRI::ORI
            | AluOPRRI::ANDI
            | AluOPRRI::SLLI
            | AluOPRRI::SRLI
            | AluOPRRI::SRAI => 0b0010011,
            AluOPRRI::ADDIW | AluOPRRI::SLLIW | AluOPRRI::SRLIW | AluOPRRI::SRAIW => 0b0011011,
        }
    }
}

impl FloatRoundingMode {
    pub fn bits(self) -> u8 {
        match self {
            FloatRoundingMode::RNE => 0b000,
            FloatRoundingMode::RTZ => 0b001,
            FloatRoundingMode::RDN => 0b010,
            FloatRoundingMode::RUP => 0b011,
            FloatRoundingMode::RMM => 0b100,
        }
    }
}

impl FloatException {
    pub(crate) fn mask(self) -> u32 {
        match self {
            FloatException::NV => 1 << 4,
            FloatException::DZ => 1 << 3,
            FloatException::OF => 1 << 2,
            FloatException::UF => 1 << 1,
            FloatException::NX => 1 << 0,
        }
    }
}

impl FClassResult {
    pub(crate) fn reuslt(self) -> u32 {
        match self {
            FClassResult::NegInfinite => 0,
            FClassResult::NegNormal => 1,
            FClassResult::NegSubNormal => 2,
            FClassResult::NegZero => 3,
            FClassResult::PosZero => 4,
            FClassResult::PosSunNormal => 5,
            FClassResult::PosNormal => 6,
            FClassResult::PosInfinite => 7,
            FClassResult::SNaN => 8,
            FClassResult::QNaN => 9,
        }
    }
}

impl LoadOP {
    pub(crate) fn op_name(self) -> String {
        get_op_name(self)
    }
    pub(crate) fn from_type(t: Type) -> Self {
        if t.is_float() {
            return if t.bits() == 32 { Self::FLW } else { Self::FLD };
        }
        match t {
            B1 | B8 => Self::LBU,
            B16 => Self::LHU,
            B32 | R32 => Self::LWU,
            B64 | R64 | I64 => Self::LD,

            I8 => Self::LB,
            I16 => Self::LH,
            I32 => Self::LW,
            _ => unreachable!(),
        }
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            Self::LB | Self::LH | Self::LW | Self::LBU | Self::LHU | Self::LWU | Self::LD => {
                0b0000011
            }
            Self::FLW | Self::FLD => 0b0000111,
        }
    }
    pub(crate) fn funct3(self) -> u32 {
        match self {
            Self::LB => 0b000,
            Self::LH => 0b001,
            Self::LW => 0b010,
            Self::LWU => 0b110,
            Self::LBU => 0b100,
            Self::LHU => 0b101,
            Self::LD => 0b011,
            Self::FLW => 0b010,
            Self::FLD => 0b011,
        }
    }
}

impl StoreOP {
    pub(crate) fn op_name(self) -> String {
        get_op_name(self)
    }
    pub(crate) fn from_type(t: Type) -> Self {
        if t.is_float() {
            return if t.bits() == 32 { Self::FSW } else { Self::FSD };
        }
        match t.bits() {
            1 | 8 => Self::SB,
            16 => Self::SH,
            32 => Self::SW,
            64 => Self::SD,
            _ => unreachable!(),
        }
    }
    pub(crate) fn op_code(self) -> u32 {
        match self {
            Self::SB | Self::SH | Self::SW | Self::SD => 0b0100011,
            Self::FSW | Self::FSD => 0b0100111,
        }
    }
    pub(crate) fn funct3(self) -> u32 {
        match self {
            Self::SB => 0b000,
            Self::SH => 0b001,
            Self::SW => 0b010,
            Self::SD => 0b011,
            Self::FSW => 0b010,
            Self::FSD => 0b011,
        }
    }
}
