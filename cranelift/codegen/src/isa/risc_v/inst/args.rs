//! AArch64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::ir::condcodes::{CondCode, FloatCC};

use super::*;

pub static WORD_SIZE: u8 = 8;

use std::fmt::{Display, Formatter, Result};
/*
    document used this term.
    short for "remain"?????????

    1100000 00000 rs1 rm rd 1010011 FCVT.W.S
    1100000 00001 rs1 rm rd 1010011 FCVT.WU.S
*/
static RM: u32 = 0;

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

    /*
        only register in AMode::RegOffset can be alloc by regalloc.
    */
    pub(crate) fn get_base_register_mut(&mut self) -> Option<&mut Reg> {
        match self {
            &mut AMode::RegOffset(ref mut reg, ..) => Some(reg),
            _ => None,
        }
    }

    pub(crate) fn get_offset_with_state(&self, state: &EmitState) -> i64 {
        match self {
            &AMode::NominalSPOffset(offset, _) => offset + state.virtual_sp_offset,
            _ => self.get_offset(),
        }
    }

    fn get_offset(&self) -> i64 {
        match self {
            &AMode::RegOffset(_, offset, ..) => offset,
            &AMode::SPOffset(offset, _) => offset,
            &AMode::FPOffset(offset, _) => offset,
            &AMode::NominalSPOffset(offset, _) => offset,
        }
    }

    pub(crate) fn to_string_may_be_with_reg_universe(
        &self,
        universe: Option<&RealRegUniverse>,
    ) -> String {
        if let Some(universe) = universe {
            let reg = self.get_base_register();
            let offset = self.get_offset();
            match self {
                &AMode::NominalSPOffset(..) => format!("{}", self),
                _ => format!("{}({})", offset, reg.show_with_rru(universe)),
            }
        } else {
            format!("{}", self)
        }
    }
}

impl Display for AMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            &AMode::RegOffset(r, offset, ..) => {
                //todo:: with RegUniverse
                write!(f, "{}({:?})", offset, r)
            }
            &AMode::SPOffset(offset, ..) => {
                write!(f, "{}(sp)", offset)
            }
            &AMode::NominalSPOffset(offset, ..) => {
                write!(f, "{}(nominal_sp)", offset)
            }
            &AMode::FPOffset(offset, ..) => {
                write!(f, "{}(fp)", offset)
            }
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

impl AluOPRRRR {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::FMADD_S => "fmadd.s",
            Self::FMSUB_S => "fmsub.s",
            Self::FNMSUB_S => "fnmsub.s",
            Self::FNMADD_S => "fnmadd.s",
            Self::FMADD_D => "fmadd.d",
            Self::FMSUB_D => "fmsub.d",
            Self::FNMSUB_D => "fnmsub.d",
            Self::FNMADD_D => "fnmadd.d",
        }
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
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::FSQRT_S => "fsqrt.s",
            Self::FCVT_W_S => "fcvt.w.s",
            Self::FCVT_WU_S => "fcvt.wu.s",
            Self::FMV_X_W => "fmv.x.w",
            Self::FCLASS_S => "fclass.s",
            Self::FCVT_S_W => "fcvt.s.w",
            Self::FCVT_S_WU => "fcvt.s.wu",
            Self::FMV_W_X => "fmv.w.x",
            Self::FCVT_L_S => "fcvt.l.s",
            Self::FCVT_LU_S => "fcvt.lu.s",
            Self::FCVT_S_L => "fcvt.s.l",
            Self::FCVT_S_LU => "fcvt.s.lu",
            Self::FCVT_L_D => "fcvt.l.d",
            Self::FCVT_LU_D => "fcvt.lu.d",
            Self::FMV_X_D => "fmv.x.d",
            Self::FCVT_D_L => "fcvt.d.l",
            Self::FCVT_D_LU => "fcvt.d.lu",
            Self::FMV_D_X => "fmv.d.x",
            Self::FSQRT_D => "fsqrt.d",
            Self::FCVT_S_D => "fcvt.s.d",
            Self::FCVT_D_S => "fcvt.d.s",
            Self::FCLASS_D => "fclass.d",
            Self::FCVT_W_D => "fcvt.w.d",
            Self::FCVT_WU_D => "fcvt.wu.d",
            Self::FCVT_D_W => "fcvt.d.w",
            Self::FCVT_D_WU => "fcvt.d.wu",
        }
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
            AluOPRR::FSQRT_S => RM,
            AluOPRR::FCVT_W_S => RM,
            AluOPRR::FCVT_WU_S => RM,
            AluOPRR::FMV_X_W => 0b000,
            AluOPRR::FCLASS_S => 0b001,
            AluOPRR::FCVT_S_W => RM,
            AluOPRR::FCVT_S_WU => RM,
            AluOPRR::FMV_W_X => 0b000,

            AluOPRR::FCVT_L_S => RM,
            AluOPRR::FCVT_LU_S => RM,
            AluOPRR::FCVT_S_L => RM,
            AluOPRR::FCVT_S_LU => RM,

            AluOPRR::FCVT_L_D => RM,
            AluOPRR::FCVT_LU_D => RM,
            AluOPRR::FMV_X_D => 0b000,
            AluOPRR::FCVT_D_L => RM,
            AluOPRR::FCVT_D_LU => RM,
            AluOPRR::FMV_D_X => 0b000,
            AluOPRR::FCVT_S_D => RM,
            AluOPRR::FCVT_D_S => RM,
            AluOPRR::FCLASS_D => 0b001,
            AluOPRR::FCVT_W_D => RM,
            AluOPRR::FCVT_WU_D => RM,
            AluOPRR::FCVT_D_W => RM,
            AluOPRR::FCVT_D_WU => RM,
            AluOPRR::FSQRT_D => RM,
        }
    }
}

impl AluOPRRR {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::ADD => "add",
            Self::SUB => "sub",
            Self::SLL => "sll",
            Self::SLT => "slt",
            Self::SLTU => "sltu",
            Self::XOR => "xor",
            Self::SRL => "srl",
            Self::SRA => "sra",
            Self::OR => "or",
            Self::AND => "and",
            Self::ADDW => "addw",
            Self::SUBW => "subw",
            Self::SLLW => "sllw",
            Self::SRLW => "srlw",
            Self::Sraw => "sraw",
            Self::MUL => "mul",
            Self::MULH => "mulh",
            Self::MULHSU => "mulhsu",
            Self::MULHU => "mulhu",
            Self::DIV => "div",
            Self::DIVU => "divu",
            Self::REM => "rem",
            Self::REMU => "remu",
            Self::MULW => "mulw",
            Self::DIVW => "divw",
            Self::DIVUW => "divuw",
            Self::REMW => "remw",
            Self::REMUW => "remuw",
            Self::FADD_S => "fadd.s",
            Self::FSUB_S => "fsub.s",
            Self::FMUL_S => "fmul.s",
            Self::FDIV_S => "fdiv.s",
            Self::FSGNJ_S => "fsgnj.s",
            Self::FSGNJN_S => "fsgnjn.s",
            Self::FSGNJX_S => "fsgnjx.s",
            Self::FMIN_S => "fmin.s",
            Self::FMAX_S => "fmax.s",
            Self::FEQ_S => "feq.s",
            Self::FLT_S => "flt.s",
            Self::FLE_S => "fle.s",
            Self::FADD_D => "fadd.d",
            Self::FSUB_D => "fsub.d",
            Self::FMUL_D => "fmul.d",
            Self::FDIV_D => "fdiv.d",
            Self::FSGNJ_D => "fsgnj.d",
            Self::FSGNJN_D => "fsgnjn.d",
            Self::FSGNJX_D => "fsgnjx.d",
            Self::FMIN_D => "fmin.d",
            Self::FMAX_D => "fmax.d",
            Self::FEQ_D => "feq.d",
            Self::FLT_D => "flt.d",
            Self::FLE_D => "fle.d",
        }
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

            AluOPRRR::FADD_S => RM,
            AluOPRRR::FSUB_S => RM,
            AluOPRRR::FMUL_S => RM,
            AluOPRRR::FDIV_S => RM,

            AluOPRRR::FSGNJ_S => 0b000,
            AluOPRRR::FSGNJN_S => 0b001,
            AluOPRRR::FSGNJX_S => 0b010,
            AluOPRRR::FMIN_S => 0b000,
            AluOPRRR::FMAX_S => 0b001,

            AluOPRRR::FEQ_S => 0b010,
            AluOPRRR::FLT_S => 0b001,
            AluOPRRR::FLE_S => 0b000,

            AluOPRRR::FADD_D => RM,
            AluOPRRR::FSUB_D => RM,
            AluOPRRR::FMUL_D => RM,
            AluOPRRR::FDIV_D => RM,

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

    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::ADDI => "addi",
            Self::SLTI => "slti",
            Self::SLTIU => "sltiu",
            Self::XORI => "xori",
            Self::ORI => "ori",
            Self::ANDI => "andi",
            Self::SLLI => "slli",
            Self::SRLI => "srli",
            Self::SRAI => "srai",
            Self::ADDIW => "addiw",
            Self::SLLIW => "slliw",
            Self::SRLIW => "srliw",
            Self::SRAIW => "sraiw",
        }
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

impl LoadOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::LB => "lb",
            Self::LH => "lh",
            Self::LW => "lw",
            Self::LBU => "lbu",
            Self::LHU => "lhu",
            Self::LWU => "lwu",
            Self::LD => "ld",
            Self::FLW => "flw",
            Self::FLD => "fld",
        }
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
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::SB => "sb",
            Self::SH => "sh",
            Self::SW => "sw",
            Self::SD => "sd",
            Self::FSW => "fsw",
            Self::FSD => "fsd",
        }
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

impl FloatFlagOp {
    // give me the option reg
    pub(crate) fn rs1(self, reg: OptionReg) -> u32 {
        // current all zero
        if let Some(r) = reg {
            r.get_hw_encoding() as u32
        } else {
            0
        }
    }
    pub(crate) fn funct3(self) -> u32 {
        match self {
            FloatFlagOp::FRCSR => 0b010,
            FloatFlagOp::FRRM => 0b010,
            FloatFlagOp::FRFLAGS => 0b010,
            FloatFlagOp::FSRMI => 0b101,
            FloatFlagOp::FSFLAGSI => 0b101,
            FloatFlagOp::FSCSR => 0b001,
            FloatFlagOp::FSRM => 0b001,
            FloatFlagOp::FSFLAGS => 0b001,
        }
    }
    pub(crate) fn use_imm12(self) -> bool {
        match self {
            FloatFlagOp::FSRMI | FloatFlagOp::FSFLAGSI => true,
            _ => false,
        }
    }
    pub(crate) fn imm12(self, imm: OptionImm12) -> u32 {
        match self {
            FloatFlagOp::FRCSR => 0b000000000011,
            FloatFlagOp::FRRM => 0b000000000010,
            FloatFlagOp::FRFLAGS => 0b000000000001,
            FloatFlagOp::FSRMI => imm.unwrap().as_u32(),
            FloatFlagOp::FSFLAGSI => imm.unwrap().as_u32(),
            FloatFlagOp::FSCSR => 0b000000000011,
            FloatFlagOp::FSRM => 0b000000000010,
            FloatFlagOp::FSFLAGS => 0b000000000001,
        }
    }

    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::FRCSR => "frcsr",
            Self::FRRM => "frrm",
            Self::FRFLAGS => "frflags",
            Self::FSRMI => "fsrmi",
            Self::FSFLAGSI => "fsflagsi",
            Self::FSCSR => "fscsr",
            Self::FSRM => "fsrm",
            Self::FSFLAGS => "fsflags",
        }
    }

    pub(crate) fn op_code(self) -> u32 {
        0b1110011
    }
}

impl FClassResult {
    pub(crate) fn bit(self) -> u32 {
        match self {
            FClassResult::NegInfinite => 1 << 0,
            FClassResult::NegNormal => 1 << 1,
            FClassResult::NegSubNormal => 1 << 2,
            FClassResult::NegZero => 1 << 3,
            FClassResult::PosZero => 1 << 4,
            FClassResult::PosSubNormal => 1 << 5,
            FClassResult::PosNormal => 1 << 6,
            FClassResult::PosInfinite => 1 << 7,
            FClassResult::SNaN => 1 << 8,
            FClassResult::QNaN => 1 << 9,
        }
    }

    pub(crate) fn is_nan_bits() -> u32 {
        Self::SNaN.bit() | Self::QNaN.bit()
    }

    pub(crate) fn is_zero_bits() -> u32 {
        Self::NegZero.bit() | Self::PosZero.bit()
    }
    pub(crate) fn is_infinite_bits() -> u32 {
        Self::PosInfinite.bit() | Self::NegInfinite.bit()
    }
}

/*
    Condition code for comparing floating point numbers.

    This condition code is used by the fcmp instruction to compare floating point values. Two IEEE floating point values relate in exactly one of four ways:

    UN - unordered when either value is NaN.
    EQ - equal numerical value.
    LT - x is less than y.
    GT - x is greater than y.
*/
pub enum FloatCCBit {
    UN,
    EQ,
    LT,
    GT,
}

impl FloatCCBit {
    #[inline(always)]
    pub(crate) fn shift(self) -> u8 {
        match self {
            FloatCCBit::UN => 0,
            FloatCCBit::EQ => 1,
            FloatCCBit::LT => 2,
            FloatCCBit::GT => 3,
        }
    }
    #[inline(always)]
    pub(crate) fn bit(self) -> u8 {
        1 << self.shift()
    }
    /*
        mask bit for floatcc
    */
    pub(crate) fn floatcc_2_mask_bits<T: Into<FloatCC>>(t: T) -> u8 {
        match t.into() {
            FloatCC::Ordered => Self::EQ.bit() | Self::LT.bit() | Self::GT.bit(),
            FloatCC::Unordered => Self::UN.bit(),
            FloatCC::Equal => Self::EQ.bit(),
            FloatCC::NotEqual => Self::UN.bit() | Self::LT.bit() | Self::GT.bit(),
            FloatCC::OrderedNotEqual => Self::LT.bit() | Self::GT.bit(),
            FloatCC::UnorderedOrEqual => Self::UN.bit() | Self::EQ.bit(),
            FloatCC::LessThan => Self::LT.bit(),
            FloatCC::LessThanOrEqual => Self::LT.bit() | Self::EQ.bit(),
            FloatCC::GreaterThan => Self::GT.bit(),
            FloatCC::GreaterThanOrEqual => Self::GT.bit() | Self::EQ.bit(),
            FloatCC::UnorderedOrLessThan => Self::UN.bit() | Self::LT.bit(),
            FloatCC::UnorderedOrLessThanOrEqual => Self::UN.bit() | Self::LT.bit() | Self::EQ.bit(),
            FloatCC::UnorderedOrGreaterThan => Self::UN.bit() | Self::GT.bit(),
            FloatCC::UnorderedOrGreaterThanOrEqual => {
                Self::UN.bit() | Self::GT.bit() | Self::EQ.bit()
            }
        }
    }
}

impl AtomicOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::LR_W => "lr.w",
            Self::SC_W => "sc.w",
            Self::AMOSWAP_W => "amoswap.w",
            Self::AMOADD_W => "amoadd.w",
            Self::AMOXOR_W => "amoxor.w",
            Self::AMOAND_W => "amoand.w",
            Self::AMOOR_W => "amoor.w",
            Self::AMOMIN_W => "amomin.w",
            Self::AMOMAX_W => "amomax.w",
            Self::AMOMINU_W => "amominu.w",
            Self::AMOMAXU_W => "amomaxu.w",
            Self::LR_D => "lr.d",
            Self::SC_D => "sc.d",
            Self::AMOSWAP_D => "amoswap.d",
            Self::AMOADD_D => "amoadd.d",
            Self::AMOXOR_D => "amoxor.d",
            Self::AMOAND_D => "amoand.d",
            Self::AMOOR_D => "amoor.d",
            Self::AMOMIN_D => "amomin.d",
            Self::AMOMAX_D => "amomax.d",
            Self::AMOMINU_D => "amominu.d",
            Self::AMOMAXU_D => "amomaxu.d",
        }
    }
    pub(crate) fn op_code(self) -> u32 {
        0b0101111
    }

    pub(crate) fn funct3(self) -> u32 {
        match self {
            AtomicOP::LR_W
            | AtomicOP::SC_W
            | AtomicOP::AMOSWAP_W
            | AtomicOP::AMOADD_W
            | AtomicOP::AMOXOR_W
            | AtomicOP::AMOAND_W
            | AtomicOP::AMOOR_W
            | AtomicOP::AMOMIN_W
            | AtomicOP::AMOMAX_W
            | AtomicOP::AMOMINU_W
            | AtomicOP::AMOMAXU_W => 0b010,

            AtomicOP::LR_D
            | AtomicOP::SC_D
            | AtomicOP::AMOSWAP_D
            | AtomicOP::AMOADD_D
            | AtomicOP::AMOXOR_D
            | AtomicOP::AMOAND_D
            | AtomicOP::AMOOR_D
            | AtomicOP::AMOMIN_D
            | AtomicOP::AMOMAX_D
            | AtomicOP::AMOMINU_D
            | AtomicOP::AMOMAXU_D => 0b011,
        }
    }
    pub(crate) fn funct5(self) -> u32 {
        match self {
            AtomicOP::LR_W => 0b00010,
            AtomicOP::SC_W => 0b00011,
            AtomicOP::AMOSWAP_W => 0b00001,
            AtomicOP::AMOADD_W => 0b00000,
            AtomicOP::AMOXOR_W => 0b00100,
            AtomicOP::AMOAND_W => 0b01100,
            AtomicOP::AMOOR_W => 0b01000,
            AtomicOP::AMOMIN_W => 0b10000,
            AtomicOP::AMOMAX_W => 0b10100,
            AtomicOP::AMOMINU_W => 0b11000,
            AtomicOP::AMOMAXU_W => 0b11100,
            AtomicOP::LR_D => 0b00010,
            AtomicOP::SC_D => 0b00011,
            AtomicOP::AMOSWAP_D => 0b00001,
            AtomicOP::AMOADD_D => 0b00000,
            AtomicOP::AMOXOR_D => 0b00100,
            AtomicOP::AMOAND_D => 0b01100,
            AtomicOP::AMOOR_D => 0b01000,
            AtomicOP::AMOMIN_D => 0b10000,
            AtomicOP::AMOMAX_D => 0b10100,
            AtomicOP::AMOMINU_D => 0b11000,
            AtomicOP::AMOMAXU_D => 0b11100,
        }
    }
}
