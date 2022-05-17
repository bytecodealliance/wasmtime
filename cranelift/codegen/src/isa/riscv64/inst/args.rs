//! AArch64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::ir::condcodes::{CondCode, FloatCC};

use super::*;

pub static WORD_SIZE: u8 = 8;

use crate::isa::riscv64::inst::reg_to_gpr_num;
use crate::machinst::*;

use std::fmt::{Display, Formatter, Result};
/*
    document used this term.
    short for "remain"?????????

    1100000 00000 rs1 rm rd 1010011 FCVT.W.S
    1100000 00001 rs1 rm rd 1010011 FCVT.WU.S
*/
static RISCV_RM_FUNCT3: u32 = 0b111; /*gnu gcc tool chain use this value.*/

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
    pub(crate) fn reg_offset(reg: Reg, imm: i64, ty: Type) -> AMode {
        AMode::RegOffset(reg, imm, ty)
    }

    pub(crate) fn get_base_register(&self) -> Reg {
        match self {
            &AMode::RegOffset(reg, ..) => reg,
            &AMode::SPOffset(..) => stack_reg(),
            &AMode::FPOffset(..) => fp_reg(),
            &AMode::NominalSPOffset(..) => stack_reg(),
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

    pub(crate) fn to_string_may_be_alloc(&self, allocs: &mut AllocationConsumer<'_>) -> String {
        let reg = self.get_base_register();
        let next = allocs.next(reg);
        let offset = self.get_offset();
        match self {
            &AMode::NominalSPOffset(..) => format!("{}", self),
            _ => format!("{}({})", offset, reg_name(next),),
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
pub struct IntegerCompare {
    pub(crate) kind: IntCC,
    pub(crate) rs1: Reg,
    pub(crate) rs2: Reg,
}

pub(crate) enum BranchFunct3 {
    // ==
    Eq,
    // !=
    Ne,
    // signed <
    Lt,
    // signed >=
    Ge,
    // unsigned <
    Ltu,
    // unsigned >=
    Geu,
}

impl BranchFunct3 {
    pub(crate) fn bits(self) -> u32 {
        match self {
            BranchFunct3::Eq => 0b000,
            BranchFunct3::Ne => 0b001,
            BranchFunct3::Lt => 0b100,
            BranchFunct3::Ge => 0b101,
            BranchFunct3::Ltu => 0b110,
            BranchFunct3::Geu => 0b111,
        }
    }
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            BranchFunct3::Eq => "eq",
            BranchFunct3::Ne => "ne",
            BranchFunct3::Lt => "lt",
            BranchFunct3::Ge => "ge",
            BranchFunct3::Ltu => "ltu",
            BranchFunct3::Geu => "geu",
        }
    }
}
impl IntegerCompare {
    pub(crate) fn op_code(self) -> u32 {
        0b1100011
    }

    /*
       funct3 and if need inverse the register
    */
    pub(crate) fn funct3(&self) -> (BranchFunct3, bool) {
        match self.kind {
            IntCC::Equal => (BranchFunct3::Eq, false),
            IntCC::NotEqual => (BranchFunct3::Ne, false),
            IntCC::SignedLessThan => (BranchFunct3::Lt, false),
            IntCC::SignedGreaterThanOrEqual => (BranchFunct3::Ge, false),

            IntCC::SignedGreaterThan => (BranchFunct3::Lt, true),
            IntCC::SignedLessThanOrEqual => (BranchFunct3::Ge, true),

            IntCC::UnsignedLessThan => (BranchFunct3::Ltu, false),
            IntCC::UnsignedGreaterThanOrEqual => (BranchFunct3::Geu, false),

            IntCC::UnsignedGreaterThan => (BranchFunct3::Ltu, true),
            IntCC::UnsignedLessThanOrEqual => (BranchFunct3::Geu, true),
            IntCC::Overflow => todo!(),
            IntCC::NotOverflow => todo!(),
        }
    }

    #[inline(always)]
    pub(crate) fn op_name(&self) -> String {
        let (f, _) = self.funct3();
        format!("b{}", f.op_name())
    }
    #[inline(always)]
    pub(crate) fn set_kind(self, kind: IntCC) -> Self {
        Self { kind, ..self }
    }
    #[inline(always)]
    pub(crate) fn register_should_inverse_when_emit(&self) -> bool {
        let (_, x) = self.funct3();
        x
    }
    pub(crate) fn emit(self) -> u32 {
        let (funct3, reverse) = self.funct3();
        let (rs1, rs2) = if reverse {
            (self.rs2, self.rs1)
        } else {
            (self.rs1, self.rs2)
        };

        self.op_code() | funct3.bits() << 12 | reg_to_gpr_num(rs1) << 15 | reg_to_gpr_num(rs2) << 20
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
            Self::FmaddS => "fmadd.s",
            Self::FmsubS => "fmsub.s",
            Self::FnmsubS => "fnmsub.s",
            Self::FnmaddS => "fnmadd.s",
            Self::FmaddD => "fmadd.d",
            Self::FmsubD => "fmsub.d",
            Self::FnmsubD => "fnmsub.d",
            Self::FnmaddD => "fnmadd.d",
        }
    }

    pub(crate) fn funct2(self) -> u32 {
        match self {
            AluOPRRRR::FmaddS | AluOPRRRR::FmsubS | AluOPRRRR::FnmsubS | AluOPRRRR::FnmaddS => 0,
            AluOPRRRR::FmaddD | AluOPRRRR::FmsubD | AluOPRRRR::FnmsubD | AluOPRRRR::FnmaddD => 1,
        }
    }

    pub(crate) fn funct3(self) -> u32 {
        RISCV_RM_FUNCT3
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            AluOPRRRR::FmaddS => 0b1000011,
            AluOPRRRR::FmsubS => 0b1000111,
            AluOPRRRR::FnmsubS => 0b1001011,
            AluOPRRRR::FnmaddS => 0b1001111,
            AluOPRRRR::FmaddD => 0b1000011,
            AluOPRRRR::FmsubD => 0b1000111,
            AluOPRRRR::FnmsubD => 0b1001011,
            AluOPRRRR::FnmaddD => 0b1001111,
        }
    }
}

impl AluOPRR {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::FsqrtS => "fsqrt.s",
            Self::FcvtWS => "fcvt.w.s",
            Self::FcvtWuS => "fcvt.wu.s",
            Self::FmvXW => "fmv.x.w",
            Self::FclassS => "fclass.s",
            Self::FcvtSw => "fcvt.s.w",
            Self::FcvtSwU => "fcvt.s.wu",
            Self::FmvWX => "fmv.w.x",
            Self::FcvtLS => "fcvt.l.s",
            Self::FcvtLuS => "fcvt.lu.s",
            Self::FcvtSL => "fcvt.s.l",
            Self::FcvtSLU => "fcvt.s.lu",
            Self::FcvtLd => "fcvt.l.d",
            Self::FcvtLuD => "fcvt.lu.d",
            Self::FmvXD => "fmv.x.d",
            Self::FcvtDL => "fcvt.d.l",
            Self::FcvtDLu => "fcvt.d.lu",
            Self::FmvDX => "fmv.d.x",
            Self::FsqrtD => "fsqrt.d",
            Self::FcvtSd => "fcvt.s.d",
            Self::FcvtDS => "fcvt.d.s",
            Self::FclassD => "fclass.d",
            Self::FcvtWD => "fcvt.w.d",
            Self::FcvtWuD => "fcvt.wu.d",
            Self::FcvtDW => "fcvt.d.w",
            Self::FcvtDWU => "fcvt.d.wu",
        }
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            AluOPRR::FsqrtS
            | AluOPRR::FcvtWS
            | AluOPRR::FcvtWuS
            | AluOPRR::FmvXW
            | AluOPRR::FclassS
            | AluOPRR::FcvtSw
            | AluOPRR::FcvtSwU
            | AluOPRR::FmvWX => 0b1010011,

            AluOPRR::FcvtLS | AluOPRR::FcvtLuS | AluOPRR::FcvtSL | AluOPRR::FcvtSLU => 0b1010011,

            AluOPRR::FcvtLd
            | AluOPRR::FcvtLuD
            | AluOPRR::FmvXD
            | AluOPRR::FcvtDL
            | AluOPRR::FcvtDLu
            | AluOPRR::FmvDX => 0b1010011,

            AluOPRR::FsqrtD
            | AluOPRR::FcvtSd
            | AluOPRR::FcvtDS
            | AluOPRR::FclassD
            | AluOPRR::FcvtWD
            | AluOPRR::FcvtWuD
            | AluOPRR::FcvtDW
            | AluOPRR::FcvtDWU => 0b1010011,
        }
    }
    /*
    todo in rs2 position.
    What should I call this.
        */
    pub(crate) fn rs2(self) -> u32 {
        match self {
            AluOPRR::FsqrtS => 0b00000,
            AluOPRR::FcvtWS => 0b00000,
            AluOPRR::FcvtWuS => 0b00001,
            AluOPRR::FmvXW => 0b00000,
            AluOPRR::FclassS => 0b00000,
            AluOPRR::FcvtSw => 0b00000,
            AluOPRR::FcvtSwU => 0b00001,
            AluOPRR::FmvWX => 0b00000,
            AluOPRR::FcvtLS => 0b00010,
            AluOPRR::FcvtLuS => 0b00011,
            AluOPRR::FcvtSL => 0b00010,
            AluOPRR::FcvtSLU => 0b00011,
            AluOPRR::FcvtLd => 0b00010,
            AluOPRR::FcvtLuD => 0b00011,
            AluOPRR::FmvXD => 0b00000,
            AluOPRR::FcvtDL => 0b00010,
            AluOPRR::FcvtDLu => 0b00011,
            AluOPRR::FmvDX => 0b00000,
            AluOPRR::FcvtSd => 0b00001,
            AluOPRR::FcvtDS => 0b00000,
            AluOPRR::FclassD => 0b00000,
            AluOPRR::FcvtWD => 0b00000,
            AluOPRR::FcvtWuD => 0b00001,
            AluOPRR::FcvtDW => 0b00000,
            AluOPRR::FcvtDWU => 0b00001,
            AluOPRR::FsqrtD => 0b00000,
        }
    }
    pub(crate) fn funct7(self) -> u32 {
        match self {
            AluOPRR::FsqrtS => 0b0101100,
            AluOPRR::FcvtWS => 0b1100000,
            AluOPRR::FcvtWuS => 0b1100000,
            AluOPRR::FmvXW => 0b1110000,
            AluOPRR::FclassS => 0b1110000,
            AluOPRR::FcvtSw => 0b1101000,
            AluOPRR::FcvtSwU => 0b1101000,
            AluOPRR::FmvWX => 0b1111000,
            AluOPRR::FcvtLS => 0b1100000,
            AluOPRR::FcvtLuS => 0b1100000,
            AluOPRR::FcvtSL => 0b1101000,
            AluOPRR::FcvtSLU => 0b1101000,
            AluOPRR::FcvtLd => 0b1100001,
            AluOPRR::FcvtLuD => 0b1100001,
            AluOPRR::FmvXD => 0b1110001,
            AluOPRR::FcvtDL => 0b1101001,
            AluOPRR::FcvtDLu => 0b1101001,
            AluOPRR::FmvDX => 0b1111001,
            AluOPRR::FcvtSd => 0b0100000,
            AluOPRR::FcvtDS => 0b0100001,
            AluOPRR::FclassD => 0b1110001,
            AluOPRR::FcvtWD => 0b1100001,
            AluOPRR::FcvtWuD => 0b1100001,
            AluOPRR::FcvtDW => 0b1101001,
            AluOPRR::FcvtDWU => 0b1101001,
            AluOPRR::FsqrtD => 0b0101101,
        }
    }

    pub(crate) fn funct3(self) -> u32 {
        match self {
            AluOPRR::FsqrtS => RISCV_RM_FUNCT3,
            AluOPRR::FcvtWS => RISCV_RM_FUNCT3,
            AluOPRR::FcvtWuS => RISCV_RM_FUNCT3,
            AluOPRR::FmvXW => 0b000,
            AluOPRR::FclassS => 0b001,
            AluOPRR::FcvtSw => RISCV_RM_FUNCT3,
            AluOPRR::FcvtSwU => RISCV_RM_FUNCT3,
            AluOPRR::FmvWX => 0b000,

            AluOPRR::FcvtLS => RISCV_RM_FUNCT3,
            AluOPRR::FcvtLuS => RISCV_RM_FUNCT3,
            AluOPRR::FcvtSL => RISCV_RM_FUNCT3,
            AluOPRR::FcvtSLU => RISCV_RM_FUNCT3,

            AluOPRR::FcvtLd => RISCV_RM_FUNCT3,
            AluOPRR::FcvtLuD => RISCV_RM_FUNCT3,
            AluOPRR::FmvXD => 0b000,
            AluOPRR::FcvtDL => RISCV_RM_FUNCT3,
            AluOPRR::FcvtDLu => RISCV_RM_FUNCT3,
            AluOPRR::FmvDX => 0b000,
            AluOPRR::FcvtSd => RISCV_RM_FUNCT3,
            AluOPRR::FcvtDS => RISCV_RM_FUNCT3,
            AluOPRR::FclassD => 0b001,
            AluOPRR::FcvtWD => RISCV_RM_FUNCT3,
            AluOPRR::FcvtWuD => RISCV_RM_FUNCT3,
            AluOPRR::FcvtDW => RISCV_RM_FUNCT3,
            /** todo::  AluOPRR::FcvtDWU not the same with the document,maybe changed..*/
            AluOPRR::FcvtDWU => 0b000,

            AluOPRR::FsqrtD => RISCV_RM_FUNCT3,
        }
    }
}

impl AluOPRRR {
    pub(crate) const fn op_name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Sll => "sll",
            Self::Slt => "slt",
            Self::SltU => "sltu",
            Self::Xor => "xor",
            Self::Srl => "srl",
            Self::Sra => "sra",
            Self::Or => "or",
            Self::And => "and",
            Self::Addw => "addw",
            Self::Subw => "subw",
            Self::Sllw => "sllw",
            Self::Srlw => "srlw",
            Self::Sraw => "sraw",
            Self::Mul => "mul",
            Self::Mulh => "mulh",
            Self::Mulhsu => "mulhsu",
            Self::Mulhu => "mulhu",
            Self::Div => "div",
            Self::DivU => "divu",
            Self::Rem => "rem",
            Self::RemU => "remu",
            Self::Mulw => "mulw",
            Self::Divw => "divw",
            Self::Divuw => "divuw",
            Self::Remw => "remw",
            Self::Remuw => "remuw",
            Self::FaddS => "fadd.s",
            Self::FsubS => "fsub.s",
            Self::FmulS => "fmul.s",
            Self::FdivS => "fdiv.s",
            Self::FsgnjS => "fsgnj.s",
            Self::FsgnjnS => "fsgnjn.s",
            Self::FsgnjxS => "fsgnjx.s",
            Self::FminS => "fmin.s",
            Self::FmaxS => "fmax.s",
            Self::FeqS => "feq.s",
            Self::FltS => "flt.s",
            Self::FleS => "fle.s",
            Self::FaddD => "fadd.d",
            Self::FsubD => "fsub.d",
            Self::FmulD => "fmul.d",
            Self::FdivD => "fdiv.d",
            Self::FsgnjD => "fsgnj.d",
            Self::FsgnjnD => "fsgnjn.d",
            Self::FsgnjxD => "fsgnjx.d",
            Self::FminD => "fmin.d",
            Self::FmaxD => "fmax.d",
            Self::FeqD => "feq.d",
            Self::FltD => "flt.d",
            Self::FleD => "fle.d",
            Self::Adduw => "add.uw",
            Self::Andn => "andn",
            Self::Bclr => "bclr",
            Self::Bext => "bext",
            Self::Binv => "binv",
            Self::Bset => "bset",
            Self::Clmul => "clmul",
            Self::Clmulh => "clmulh",
            Self::Clmulr => "clmulr",
            Self::Max => "max",
            Self::Maxu => "maxu",
            Self::Min => "min",
            Self::Minu => "minu",
            Self::Orn => "orn",
            Self::Rol => "rol",
            Self::Rolw => "rolw",
            Self::Ror => "ror",
            Self::Rorw => "rorw",
            Self::Sh1add => "sh1add",
            Self::Sh1adduw => "sh1add.uw",
            Self::Sh2add => "sh2add",
            Self::Sh2adduw => "sh2add.uw",
            Self::Sh3add => "sh3add",
            Self::Sh3adduw => "sh3add.uw",
            Self::Xnor => "xnor",
        }
    }

    pub fn funct3(self) -> u32 {
        match self {
            AluOPRRR::Add => 0b000,
            AluOPRRR::Sll => 0b001,
            AluOPRRR::Slt => 0b010,
            AluOPRRR::SltU => 0b011,
            AluOPRRR::Xor => 0b100,
            AluOPRRR::Srl => 0b101,
            AluOPRRR::Sra => 0b101,
            AluOPRRR::Or => 0b110,
            AluOPRRR::And => 0b111,
            AluOPRRR::Sub => 0b000,

            AluOPRRR::Addw => 0b000,
            AluOPRRR::Subw => 0b000,
            AluOPRRR::Sllw => 0b001,
            AluOPRRR::Srlw => 0b101,
            AluOPRRR::Sraw => 0b101,

            AluOPRRR::Mul => 0b000,
            AluOPRRR::Mulh => 0b001,
            AluOPRRR::Mulhsu => 0b010,
            AluOPRRR::Mulhu => 0b011,
            AluOPRRR::Div => 0b100,
            AluOPRRR::DivU => 0b101,
            AluOPRRR::Rem => 0b110,
            AluOPRRR::RemU => 0b111,

            AluOPRRR::Mulw => 0b000,
            AluOPRRR::Divw => 0b100,
            AluOPRRR::Divuw => 0b101,
            AluOPRRR::Remw => 0b110,
            AluOPRRR::Remuw => 0b111,

            AluOPRRR::FaddS => RISCV_RM_FUNCT3,
            AluOPRRR::FsubS => RISCV_RM_FUNCT3,
            AluOPRRR::FmulS => RISCV_RM_FUNCT3,
            AluOPRRR::FdivS => RISCV_RM_FUNCT3,

            AluOPRRR::FsgnjS => 0b000,
            AluOPRRR::FsgnjnS => 0b001,
            AluOPRRR::FsgnjxS => 0b010,
            AluOPRRR::FminS => 0b000,
            AluOPRRR::FmaxS => 0b001,

            AluOPRRR::FeqS => 0b010,
            AluOPRRR::FltS => 0b001,
            AluOPRRR::FleS => 0b000,

            AluOPRRR::FaddD => RISCV_RM_FUNCT3,
            AluOPRRR::FsubD => RISCV_RM_FUNCT3,
            AluOPRRR::FmulD => RISCV_RM_FUNCT3,
            AluOPRRR::FdivD => RISCV_RM_FUNCT3,

            AluOPRRR::FsgnjD => 0b000,
            AluOPRRR::FsgnjnD => 0b001,
            AluOPRRR::FsgnjxD => 0b010,
            AluOPRRR::FminD => 0b000,
            AluOPRRR::FmaxD => 0b001,
            AluOPRRR::FeqD => 0b010,
            AluOPRRR::FltD => 0b001,
            AluOPRRR::FleD => 0b000,
            AluOPRRR::Adduw => 0b000,
            AluOPRRR::Andn => 0b111,
            AluOPRRR::Bclr => 0b001,
            AluOPRRR::Bext => 0b101,
            AluOPRRR::Binv => 0b001,
            AluOPRRR::Bset => 0b001,
            AluOPRRR::Clmul => 0b001,
            AluOPRRR::Clmulh => 0b011,
            AluOPRRR::Clmulr => 0b010,
            AluOPRRR::Max => 0b110,
            AluOPRRR::Maxu => 0b111,
            AluOPRRR::Min => 0b100,
            AluOPRRR::Minu => 0b101,
            AluOPRRR::Orn => 0b110,
            AluOPRRR::Rol => 0b001,
            AluOPRRR::Rolw => 0b001,
            AluOPRRR::Ror => 0b101,
            AluOPRRR::Rorw => 0b101,
            AluOPRRR::Sh1add => 0b010,
            AluOPRRR::Sh1adduw => 0b010,
            AluOPRRR::Sh2add => 0b100,
            AluOPRRR::Sh2adduw => 0b100,
            AluOPRRR::Sh3add => 0b110,
            AluOPRRR::Sh3adduw => 0b110,
            AluOPRRR::Xnor => 0b100,
        }
    }

    pub fn op_code(self) -> u32 {
        match self {
            AluOPRRR::Add
            | AluOPRRR::Sub
            | AluOPRRR::Sll
            | AluOPRRR::Slt
            | AluOPRRR::SltU
            | AluOPRRR::Xor
            | AluOPRRR::Srl
            | AluOPRRR::Sra
            | AluOPRRR::Or
            | AluOPRRR::And => 0b0110011,

            AluOPRRR::Addw | AluOPRRR::Subw | AluOPRRR::Sllw | AluOPRRR::Srlw | AluOPRRR::Sraw => {
                0b0111011
            }

            AluOPRRR::Mul
            | AluOPRRR::Mulh
            | AluOPRRR::Mulhsu
            | AluOPRRR::Mulhu
            | AluOPRRR::Div
            | AluOPRRR::DivU
            | AluOPRRR::Rem
            | AluOPRRR::RemU => 0b0110011,

            AluOPRRR::Mulw
            | AluOPRRR::Divw
            | AluOPRRR::Divuw
            | AluOPRRR::Remw
            | AluOPRRR::Remuw => 0b0111011,

            AluOPRRR::FaddS
            | AluOPRRR::FsubS
            | AluOPRRR::FmulS
            | AluOPRRR::FdivS
            | AluOPRRR::FsgnjS
            | AluOPRRR::FsgnjnS
            | AluOPRRR::FsgnjxS
            | AluOPRRR::FminS
            | AluOPRRR::FmaxS
            | AluOPRRR::FeqS
            | AluOPRRR::FltS
            | AluOPRRR::FleS => 0b1010011,

            AluOPRRR::FaddD
            | AluOPRRR::FsubD
            | AluOPRRR::FmulD
            | AluOPRRR::FdivD
            | AluOPRRR::FsgnjD
            | AluOPRRR::FsgnjnD
            | AluOPRRR::FsgnjxD
            | AluOPRRR::FminD
            | AluOPRRR::FmaxD
            | AluOPRRR::FeqD
            | AluOPRRR::FltD
            | AluOPRRR::FleD => 0b1010011,

            AluOPRRR::Adduw => 0b0111011,
            AluOPRRR::Andn
            | AluOPRRR::Bclr
            | AluOPRRR::Bext
            | AluOPRRR::Binv
            | AluOPRRR::Bset
            | AluOPRRR::Clmul
            | AluOPRRR::Clmulh
            | AluOPRRR::Clmulr
            | AluOPRRR::Max
            | AluOPRRR::Maxu
            | AluOPRRR::Min
            | AluOPRRR::Minu
            | AluOPRRR::Orn
            | AluOPRRR::Rol
            | AluOPRRR::Ror
            | AluOPRRR::Sh1add
            | AluOPRRR::Sh2add
            | AluOPRRR::Sh3add
            | AluOPRRR::Xnor => 0b0110011,

            AluOPRRR::Rolw
            | AluOPRRR::Rorw
            | AluOPRRR::Sh2adduw
            | AluOPRRR::Sh3adduw
            | AluOPRRR::Sh1adduw => 0b0111011,
        }
    }

    pub const fn funct7(self) -> u32 {
        match self {
            AluOPRRR::Add => 0b0000000,
            AluOPRRR::Sub => 0b0100000,
            AluOPRRR::Sll => 0b0000000,
            AluOPRRR::Slt => 0b0000000,
            AluOPRRR::SltU => 0b0000000,
            AluOPRRR::Xor => 0b0000000,
            AluOPRRR::Srl => 0b0000000,
            AluOPRRR::Sra => 0b0100000,
            AluOPRRR::Or => 0b0000000,
            AluOPRRR::And => 0b0000000,

            AluOPRRR::Addw => 0b0000000,
            AluOPRRR::Subw => 0b0100000,
            AluOPRRR::Sllw => 0b0000000,
            AluOPRRR::Srlw => 0b0000000,
            AluOPRRR::Sraw => 0b0100000,

            AluOPRRR::Mul => 0b0000001,
            AluOPRRR::Mulh => 0b0000001,
            AluOPRRR::Mulhsu => 0b0000001,
            AluOPRRR::Mulhu => 0b0000001,
            AluOPRRR::Div => 0b0000001,
            AluOPRRR::DivU => 0b0000001,
            AluOPRRR::Rem => 0b0000001,
            AluOPRRR::RemU => 0b0000001,

            AluOPRRR::Mulw => 0b0000001,
            AluOPRRR::Divw => 0b0000001,
            AluOPRRR::Divuw => 0b0000001,
            AluOPRRR::Remw => 0b0000001,
            AluOPRRR::Remuw => 0b0000001,

            AluOPRRR::FaddS => 0b0000000,
            AluOPRRR::FsubS => 0b0000100,
            AluOPRRR::FmulS => 0b0001000,
            AluOPRRR::FdivS => 0b0001100,

            AluOPRRR::FsgnjS => 0b0010000,
            AluOPRRR::FsgnjnS => 0b0010000,
            AluOPRRR::FsgnjxS => 0b0010000,
            AluOPRRR::FminS => 0b0010100,
            AluOPRRR::FmaxS => 0b0010100,
            AluOPRRR::FeqS => 0b1010000,
            AluOPRRR::FltS => 0b1010000,
            AluOPRRR::FleS => 0b1010000,
            AluOPRRR::FaddD => 0b0000001,
            AluOPRRR::FsubD => 0b0000101,
            AluOPRRR::FmulD => 0b0001001,
            AluOPRRR::FdivD => 0b0001101,

            AluOPRRR::FsgnjD => 0b0010001,
            AluOPRRR::FsgnjnD => 0b0010001,
            AluOPRRR::FsgnjxD => 0b0010001,
            AluOPRRR::FminD => 0b0010101,
            AluOPRRR::FmaxD => 0b0010101,
            AluOPRRR::FeqD => 0b1010001,
            AluOPRRR::FltD => 0b1010001,
            AluOPRRR::FleD => 0b1010001,

            AluOPRRR::Adduw => 0b0000100,
            AluOPRRR::Andn => 0b0100000,
            AluOPRRR::Bclr => 0b0100100,
            AluOPRRR::Bext => 0b0100100,
            AluOPRRR::Binv => 0b0110100,
            AluOPRRR::Bset => 0b0010100,
            AluOPRRR::Clmul => 0b0000101,
            AluOPRRR::Clmulh => 0b0000101,
            AluOPRRR::Clmulr => 0b0000101,
            AluOPRRR::Max => 0b0000101,
            AluOPRRR::Maxu => 0b0000101,
            AluOPRRR::Min => 0b0000101,
            AluOPRRR::Minu => 0b0000101,
            AluOPRRR::Orn => 0b0100000,
            AluOPRRR::Rol => 0b0110000,
            AluOPRRR::Rolw => 0b0110000,
            AluOPRRR::Ror => 0b0110000,
            AluOPRRR::Rorw => 0b0110000,
            AluOPRRR::Sh1add => 0b0010000,
            AluOPRRR::Sh1adduw => 0b0010000,
            AluOPRRR::Sh2add => 0b0010000,
            AluOPRRR::Sh2adduw => 0b0010000,
            AluOPRRR::Sh3add => 0b0010000,
            AluOPRRR::Sh3adduw => 0b0010000,
            AluOPRRR::Xnor => 0b0100000,
        }
    }
}

impl AluOPRRI {
    /*
        int 64bit this is 6 bit length, otherwise is 7 bit length
    */
    pub(crate) fn option_funct6(self) -> Option<u32> {
        match self {
            AluOPRRI::Slli => Some(0b00_0000),
            AluOPRRI::Srli => Some(0b00_0000),
            AluOPRRI::Srai => Some(0b01_0000),
            _ => None,
        }
    }
    /*
        Slliw .. etc operation on 32-bit value , only need 5-bite shift size.
    */
    pub(crate) fn option_funct7(self) -> Option<u32> {
        match self {
            Self::Slliw => Some(0b000_0000),
            Self::SrliW => Some(0b000_0000),
            Self::Sraiw => Some(0b010_0000),
            _ => None,
        }
    }

    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::Addi => "addi",
            Self::Slti => "slti",
            Self::SltiU => "sltiu",
            Self::Xori => "xori",
            Self::Ori => "ori",
            Self::Andi => "andi",
            Self::Slli => "slli",
            Self::Srli => "srli",
            Self::Srai => "srai",
            Self::Addiw => "addiw",
            Self::Slliw => "slliw",
            Self::SrliW => "srliw",
            Self::Sraiw => "sraiw",
            Self::Bclri => "bclri",
            Self::Bexti => "bexti",
            Self::Binvi => "binvi",
            Self::Bseti => "bseti",
            Self::Rori => "rori",
            Self::Roriw => "roriw",
            Self::SlliUw => "slli.uw",
            Self::Clz => "clz",
            Self::Clzw => "clzw",
            Self::Cpop => "cpop",
            Self::Cpopw => "cpopw",
            Self::Ctz => "ctz",
            Self::Ctzw => "ctzw",
            Self::Rev8 => "rev8",
            Self::Sextb => "sext.b",
            Self::Sexth => "sext.h",
            Self::Zexth => "zext.h",
            Self::Orcb => "orc.b",
        }
    }

    pub fn funct3(self) -> u32 {
        match self {
            AluOPRRI::Addi => 0b000,
            AluOPRRI::Slti => 0b010,
            AluOPRRI::SltiU => 0b011,
            AluOPRRI::Xori => 0b100,
            AluOPRRI::Ori => 0b110,
            AluOPRRI::Andi => 0b111,
            AluOPRRI::Slli => 0b001,
            AluOPRRI::Srli => 0b101,
            AluOPRRI::Srai => 0b101,
            AluOPRRI::Addiw => 0b000,
            AluOPRRI::Slliw => 0b001,
            AluOPRRI::SrliW => 0b101,
            AluOPRRI::Sraiw => 0b101,
            AluOPRRI::Bclri => 0b001,
            AluOPRRI::Bexti => 0b101,
            AluOPRRI::Binvi => 0b001,
            AluOPRRI::Bseti => 0b001,
            AluOPRRI::Rori => 0b101,
            AluOPRRI::Roriw => 0b101,
            AluOPRRI::SlliUw => 0b001,
            AluOPRRI::Clz => 0b001,
            AluOPRRI::Clzw => 0b001,
            AluOPRRI::Cpop => 0b001,
            AluOPRRI::Cpopw => 0b001,
            AluOPRRI::Ctz => 0b001,
            AluOPRRI::Ctzw => 0b001,
            AluOPRRI::Rev8 => 0b101,
            AluOPRRI::Sextb => 0b001,
            AluOPRRI::Sexth => 0b001,
            AluOPRRI::Zexth => 0b100,
            AluOPRRI::Orcb => 0b101,
        }
    }

    pub fn op_code(self) -> u32 {
        match self {
            AluOPRRI::Addi
            | AluOPRRI::Slti
            | AluOPRRI::SltiU
            | AluOPRRI::Xori
            | AluOPRRI::Ori
            | AluOPRRI::Andi
            | AluOPRRI::Slli
            | AluOPRRI::Srli
            | AluOPRRI::Srai
            | AluOPRRI::Bclri
            | AluOPRRI::Bexti
            | AluOPRRI::Binvi
            | AluOPRRI::Bseti
            | AluOPRRI::Rori
            | AluOPRRI::Clz
            | AluOPRRI::Cpop
            | AluOPRRI::Ctz
            | AluOPRRI::Rev8
            | AluOPRRI::Sextb
            | AluOPRRI::Sexth
            | AluOPRRI::Orcb => 0b0010011,

            AluOPRRI::Addiw
            | AluOPRRI::Slliw
            | AluOPRRI::SrliW
            | AluOPRRI::Sraiw
            | AluOPRRI::Roriw
            | AluOPRRI::SlliUw
            | AluOPRRI::Clzw
            | AluOPRRI::Cpopw
            | AluOPRRI::Ctzw => 0b0011011,
            AluOPRRI::Zexth => 0b0111011,
        }
    }

    pub(crate) fn is_bit_manip(self) -> bool {
        match self {
            Self::Bclri
            | Self::Bexti
            | Self::Binvi
            | Self::Bseti
            | Self::Rori
            | Self::Roriw
            | Self::SlliUw
            | Self::Clz
            | Self::Clzw
            | Self::Cpop
            | Self::Cpopw
            | Self::Ctz
            | Self::Ctzw
            | Self::Rev8
            | Self::Sextb
            | Self::Sexth
            | Self::Zexth
            | Self::Orcb => true,
            _ => false,
        }
    }

    /*
        return shamt size
    */
    pub(crate) fn need_shamt(self) -> Option<u8> {
        match self {
            Self::Bclri => Some(6),
            Self::Bexti => Some(6),
            Self::Binvi => Some(6),
            Self::Bseti => Some(6),
            Self::Rori => Some(6),
            Self::Roriw => Some(5),
            Self::SlliUw => Some(6),
            _ => None,
        }
    }

    pub(crate) fn shamt_mask(x: u8) -> u8 {
        match x {
            5 => 0b1_1111,
            6 => 0b11_1111,
            _ => unreachable!(),
        }
    }

    /*
        some instruction use imm12 for function code.
    */
    pub(crate) fn funct12(self, shamt: Option<u8>) -> Imm12 {
        if self.need_shamt().is_some() {
            assert!(shamt.is_some());
        } else {
            assert!(shamt.is_none());
        }
        let shamt = shamt.map(|s| s as u32);
        let bits: u32 = match self {
            Self::Bclri => shamt.unwrap() | 0b010010 << 6,
            Self::Bexti => shamt.unwrap() | 0b010010 << 6,
            Self::Binvi => shamt.unwrap() | 0b011010 << 6,
            Self::Bseti => shamt.unwrap() | 0b001010 << 6,
            Self::Rori => shamt.unwrap() | 0b011000 << 6,
            Self::Roriw => shamt.unwrap() | 0b0110000 << 5,
            Self::SlliUw => shamt.unwrap() | 0b000010 << 6,
            Self::Clz => 0b011000000000,
            Self::Clzw => 0b011000000000,
            Self::Cpop => 0b011000000010,
            Self::Cpopw => 0b011000000010,
            Self::Ctz => 0b011000000001,
            Self::Ctzw => 0b011000000001,
            Self::Rev8 => 0b011010111000,
            Self::Sextb => 0b011000000100,
            Self::Sexth => 0b011000000101,
            Self::Zexth => 0b000010000000,
            Self::Orcb => 0b001010000111,
            _ => unreachable!(),
        };
        Imm12::from_bits(bits as i16)
    }
}

impl FloatRoundingMode {
    #[inline(always)]
    pub fn bits(self) -> u8 {
        match self {
            FloatRoundingMode::RNE => 0b000,
            FloatRoundingMode::RTZ => 0b001,
            FloatRoundingMode::RDN => 0b010,
            FloatRoundingMode::RUP => 0b011,
            FloatRoundingMode::RMM => 0b100,
        }
    }
    /*
        use to FSRMI to set rounding mod
    */
    #[inline(always)]
    pub(crate) fn to_imm12(self) -> Imm12 {
        Imm12::from_bits(self.bits() as i16)
    }
}

impl FFlagsException {
    #[inline(always)]
    pub(crate) fn mask(self) -> u32 {
        match self {
            FFlagsException::NV => 1 << 4,
            FFlagsException::DZ => 1 << 3,
            FFlagsException::OF => 1 << 2,
            FFlagsException::UF => 1 << 1,
            FFlagsException::NX => 1 << 0,
        }
    }
}

impl LoadOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::Lb => "lb",
            Self::Lh => "lh",
            Self::Lw => "lw",
            Self::Lbu => "lbu",
            Self::Lhu => "lhu",
            Self::Lwu => "lwu",
            Self::Ld => "ld",
            Self::Flw => "flw",
            Self::Fld => "fld",
        }
    }

    pub(crate) fn from_type(t: Type) -> Self {
        if t.is_float() {
            return if t == F32 { Self::Flw } else { Self::Fld };
        }
        match t {
            B1 | B8 => Self::Lbu,
            B16 => Self::Lhu,
            B32 | R32 => Self::Lwu,
            B64 | R64 | I64 => Self::Ld,

            I8 => Self::Lb,
            I16 => Self::Lh,
            I32 => Self::Lw,
            _ => unreachable!(),
        }
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            Self::Lb | Self::Lh | Self::Lw | Self::Lbu | Self::Lhu | Self::Lwu | Self::Ld => {
                0b0000011
            }
            Self::Flw | Self::Fld => 0b0000111,
        }
    }
    pub(crate) fn funct3(self) -> u32 {
        match self {
            Self::Lb => 0b000,
            Self::Lh => 0b001,
            Self::Lw => 0b010,
            Self::Lwu => 0b110,
            Self::Lbu => 0b100,
            Self::Lhu => 0b101,
            Self::Ld => 0b011,
            Self::Flw => 0b010,
            Self::Fld => 0b011,
        }
    }
}

impl StoreOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::Sb => "sb",
            Self::Sh => "sh",
            Self::Sw => "sw",
            Self::Sd => "sd",
            Self::Fsw => "fsw",
            Self::Fsd => "fsd",
        }
    }
    pub(crate) fn from_type(t: Type) -> Self {
        if t.is_float() {
            return if t == F32 { Self::Fsw } else { Self::Fsd };
        }
        match t.bits() {
            1 | 8 => Self::Sb,
            16 => Self::Sh,
            32 => Self::Sw,
            64 => Self::Sd,
            _ => unreachable!(),
        }
    }
    pub(crate) fn op_code(self) -> u32 {
        match self {
            Self::Sb | Self::Sh | Self::Sw | Self::Sd => 0b0100011,
            Self::Fsw | Self::Fsd => 0b0100111,
        }
    }
    pub(crate) fn funct3(self) -> u32 {
        match self {
            Self::Sb => 0b000,
            Self::Sh => 0b001,
            Self::Sw => 0b010,
            Self::Sd => 0b011,
            Self::Fsw => 0b010,
            Self::Fsd => 0b011,
        }
    }
}

impl FloatFlagOp {
    pub(crate) fn funct3(self) -> u32 {
        match self {
            FloatFlagOp::Frcsr => 0b010,
            FloatFlagOp::Frrm => 0b010,
            FloatFlagOp::Frflags => 0b010,
            FloatFlagOp::Fsrmi => 0b101,
            FloatFlagOp::Fsflagsi => 0b101,
            FloatFlagOp::Fscsr => 0b001,
            FloatFlagOp::Fsrm => 0b001,
            FloatFlagOp::Fsflags => 0b001,
        }
    }
    pub(crate) fn use_imm12(self) -> bool {
        match self {
            FloatFlagOp::Fsrmi | FloatFlagOp::Fsflagsi => true,
            _ => false,
        }
    }
    pub(crate) fn imm12(self, imm: OptionImm12) -> u32 {
        match self {
            FloatFlagOp::Frcsr => 0b000000000011,
            FloatFlagOp::Frrm => 0b000000000010,
            FloatFlagOp::Frflags => 0b000000000001,
            FloatFlagOp::Fsrmi => imm.unwrap().as_u32(),
            FloatFlagOp::Fsflagsi => imm.unwrap().as_u32(),
            FloatFlagOp::Fscsr => 0b000000000011,
            FloatFlagOp::Fsrm => 0b000000000010,
            FloatFlagOp::Fsflags => 0b000000000001,
        }
    }

    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::Frcsr => "frcsr",
            Self::Frrm => "frrm",
            Self::Frflags => "frflags",
            Self::Fsrmi => "fsrmi",
            Self::Fsflagsi => "fsflagsi",
            Self::Fscsr => "fscsr",
            Self::Fsrm => "fsrm",
            Self::Fsflags => "fsflags",
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
#[derive(Clone)]
pub enum FloatCCBit {
    UN,
    EQ,
    LT,
    GT,
    C(u8),
}

impl FloatCCBit {
    #[inline(always)]
    pub(crate) fn bit(&self) -> u8 {
        match self {
            FloatCCBit::UN => 1 << 0,
            FloatCCBit::EQ => 1 << 1,
            FloatCCBit::LT => 1 << 2,
            FloatCCBit::GT => 1 << 3,
            FloatCCBit::C(x) => *x,
        }
    }

    /*
        mask bit for floatcc
    */
    pub(crate) fn floatcc_2_mask_bits<T: Into<FloatCC>>(t: T) -> Self {
        let v = match t.into() {
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
        };
        Self::C(v)
    }

    #[inline(always)]
    pub(crate) fn test(&self, o: Self) -> bool {
        (self.bit() & o.bit()) != 0
    }
    /*
       there compare condition can be implemented by just one risc-v instruction.
    */
    #[inline(always)]
    pub(crate) fn just_eq(&self) -> bool {
        match self {
            Self::C(_) => self.test(Self::EQ) && self.clone().clean(Self::EQ).is_zero(),
            _ => false,
        }
    }
    #[inline(always)]
    pub(crate) fn just_lt(&self) -> bool {
        match self {
            Self::C(_) => self.test(Self::LT) && self.clone().clean(Self::LT).is_zero(),
            _ => false,
        }
    }
    #[inline(always)]
    pub(crate) fn just_le(&self) -> bool {
        match self {
            Self::C(_) => {
                (self.test(Self::LT) && self.test(Self::EQ))
                    && self.clone().clean(Self::LT).clean(Self::EQ).is_zero()
            }
            _ => false,
        }
    }
    #[inline(always)]
    fn clean(mut self, o: Self) -> Self {
        match self {
            Self::C(ref mut x) => *x = *x & !(o.bit()),
            _ => unreachable!(),
        }
        self
    }
    #[inline(always)]
    fn is_zero(&self) -> bool {
        match self {
            Self::C(x) => *x == 0,
            _ => false,
        }
    }
}

impl AtomicOP {
    #[inline(always)]
    pub(crate) fn is_load(self) -> bool {
        match self {
            Self::LrW | Self::LrD => true,
            _ => false,
        }
    }
    #[inline(always)]
    pub(crate) fn is_store(self) -> bool {
        match self {
            Self::ScW | Self::ScD => true,
            _ => false,
        }
    }
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            Self::LrW => "lr.w",
            Self::ScW => "sc.w",
            Self::AmoswapW => "amoswap.w",
            Self::AmoaddW => "amoadd.w",
            Self::AmoxorW => "amoxor.w",
            Self::AmoandW => "amoand.w",
            Self::AmoorW => "amoor.w",
            Self::AmominW => "amomin.w",
            Self::AmomaxW => "amomax.w",
            Self::AmominuW => "amominu.w",
            Self::AmomaxuW => "amomaxu.w",
            Self::LrD => "lr.d",
            Self::ScD => "sc.d",
            Self::AmoswapD => "amoswap.d",
            Self::AmoaddD => "amoadd.d",
            Self::AmoxorD => "amoxor.d",
            Self::AmoandD => "amoand.d",
            Self::AmoorD => "amoor.d",
            Self::AmominD => "amomin.d",
            Self::AmomaxD => "amomax.d",
            Self::AmominuD => "amominu.d",
            Self::AmomaxuD => "amomaxu.d",
        }
    }
    #[inline(always)]
    pub(crate) fn op_code(self) -> u32 {
        0b0101111
    }
    #[inline(always)]
    pub(crate) fn funct7(self, aq: bool, rl: bool) -> u32 {
        let mut x = self.funct5() << 2;
        if rl {
            x |= 1;
        }
        if aq {
            x |= 1 << 1;
        }
        x
    }

    pub(crate) fn funct3(self) -> u32 {
        match self {
            AtomicOP::LrW
            | AtomicOP::ScW
            | AtomicOP::AmoswapW
            | AtomicOP::AmoaddW
            | AtomicOP::AmoxorW
            | AtomicOP::AmoandW
            | AtomicOP::AmoorW
            | AtomicOP::AmominW
            | AtomicOP::AmomaxW
            | AtomicOP::AmominuW
            | AtomicOP::AmomaxuW => 0b010,

            AtomicOP::LrD
            | AtomicOP::ScD
            | AtomicOP::AmoswapD
            | AtomicOP::AmoaddD
            | AtomicOP::AmoxorD
            | AtomicOP::AmoandD
            | AtomicOP::AmoorD
            | AtomicOP::AmominD
            | AtomicOP::AmomaxD
            | AtomicOP::AmominuD
            | AtomicOP::AmomaxuD => 0b011,
        }
    }
    pub(crate) fn funct5(self) -> u32 {
        match self {
            AtomicOP::LrW => 0b00010,
            AtomicOP::ScW => 0b00011,
            AtomicOP::AmoswapW => 0b00001,
            AtomicOP::AmoaddW => 0b00000,
            AtomicOP::AmoxorW => 0b00100,
            AtomicOP::AmoandW => 0b01100,
            AtomicOP::AmoorW => 0b01000,
            AtomicOP::AmominW => 0b10000,
            AtomicOP::AmomaxW => 0b10100,
            AtomicOP::AmominuW => 0b11000,
            AtomicOP::AmomaxuW => 0b11100,
            AtomicOP::LrD => 0b00010,
            AtomicOP::ScD => 0b00011,
            AtomicOP::AmoswapD => 0b00001,
            AtomicOP::AmoaddD => 0b00000,
            AtomicOP::AmoxorD => 0b00100,
            AtomicOP::AmoandD => 0b01100,
            AtomicOP::AmoorD => 0b01000,
            AtomicOP::AmominD => 0b10000,
            AtomicOP::AmomaxD => 0b10100,
            AtomicOP::AmominuD => 0b11000,
            AtomicOP::AmomaxuD => 0b11100,
        }
    }

    pub(crate) fn from_atomicrmw_type_and_op(ty: Type, op: crate::ir::AtomicRmwOp) -> Option<Self> {
        let type_32 = ty.bits() != 64;
        match op {
            crate::ir::AtomicRmwOp::Add => {
                if type_32 {
                    Some(Self::AmoaddW)
                } else {
                    Some(Self::AmoaddD)
                }
            }
            crate::ir::AtomicRmwOp::Sub => None,
            crate::ir::AtomicRmwOp::And => {
                if type_32 {
                    Some(Self::AmoandW)
                } else {
                    Some(Self::AmoandD)
                }
            }
            crate::ir::AtomicRmwOp::Nand => None,
            crate::ir::AtomicRmwOp::Or => {
                if type_32 {
                    Some(Self::AmoorW)
                } else {
                    Some(Self::AmoorD)
                }
            }
            crate::ir::AtomicRmwOp::Xor => {
                if type_32 {
                    Some(Self::AmoxorW)
                } else {
                    Some(Self::AmoxorD)
                }
            }
            crate::ir::AtomicRmwOp::Xchg => {
                if type_32 {
                    Some(Self::AmoswapW)
                } else {
                    Some(Self::AmoswapD)
                }
            }
            crate::ir::AtomicRmwOp::Umin => {
                if type_32 {
                    Some(Self::AmominuW)
                } else {
                    Some(Self::AmominuD)
                }
            }
            crate::ir::AtomicRmwOp::Umax => {
                if type_32 {
                    Some(Self::AmomaxuW)
                } else {
                    Some(Self::AmomaxuD)
                }
            }
            crate::ir::AtomicRmwOp::Smin => {
                if type_32 {
                    Some(Self::AmominW)
                } else {
                    Some(Self::AmominD)
                }
            }
            crate::ir::AtomicRmwOp::Smax => {
                if type_32 {
                    Some(Self::AmomaxW)
                } else {
                    Some(Self::AmomaxD)
                }
            }
        }
    }
}

impl IntSelectOP {
    #[inline(always)]
    pub(crate) fn from_ir_op(op: crate::ir::Opcode) -> Self {
        match op {
            crate::ir::Opcode::Imax => Self::Imax,
            crate::ir::Opcode::Umax => Self::Umax,
            crate::ir::Opcode::Imin => Self::Imin,
            crate::ir::Opcode::Umin => Self::Umin,
            _ => unreachable!(),
        }
    }
    #[inline(always)]
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            IntSelectOP::Imax => "imax",
            IntSelectOP::Umax => "umax",
            IntSelectOP::Imin => "imin",
            IntSelectOP::Umin => "umin",
        }
    }
    #[inline(always)]
    pub(crate) fn to_int_cc(self) -> IntCC {
        match self {
            IntSelectOP::Imax => IntCC::SignedGreaterThan,
            IntSelectOP::Umax => IntCC::UnsignedGreaterThan,
            IntSelectOP::Imin => IntCC::SignedLessThan,
            IntSelectOP::Umin => IntCC::UnsignedLessThan,
        }
    }
}

impl ReferenceValidOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            ReferenceValidOP::IsNull => "is_null",
            ReferenceValidOP::IsInvalid => "is_invalid",
        }
    }
    #[inline(always)]
    pub(crate) fn from_ir_op(op: crate::ir::Opcode) -> Self {
        match op {
            crate::ir::Opcode::IsInvalid => Self::IsInvalid,
            crate::ir::Opcode::IsNull => Self::IsNull,
            _ => unreachable!(),
        }
    }
}

#[inline(always)]
pub fn is_int_and_type_signed(ty: Type) -> bool {
    ty.is_int() && is_type_signed(ty)
}
#[inline(always)]
pub fn is_type_signed(ty: Type) -> bool {
    assert!(ty.is_int());
    ty == I8 || ty == I16 || ty == I32 || ty == I64 || ty == I128
}

#[inline(always)]
pub(crate) fn ir_iflags_conflict(op: crate::ir::Opcode) {
    unreachable!("ir {} conflict with risc-v iflags", op)
}
#[inline(always)]
pub(crate) fn ir_fflags_conflict(op: crate::ir::Opcode) {
    unreachable!("ir {} conflict with risc-v fflags", op)
}

pub(crate) struct FFlags {
    /*

    The Floating-Point Control and Status Register, fcsr, is a RISC-V control and status register (CSR). The register selects the dynamic rounding mode for floating-point arithmetic operations and holds the accrued exception flags.

      todo:: e holds exception flags, so you must cannot set it, right???
        */
    // e: FFlagsException,
    r: FloatRoundingMode,
}
impl FFlags {
    pub(crate) fn new(r: FloatRoundingMode) -> Self {
        Self { r }
    }

    #[inline(always)]
    pub(crate) fn to_imm12(self) -> Imm12 {
        Imm12::from_bits(self.r.to_imm12().bits << 5)
    }
}

impl I128ArithmeticOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            I128ArithmeticOP::Add => "add_i128",
            I128ArithmeticOP::Sub => "sub_i128",
            I128ArithmeticOP::Mul => "mul_i128",
            I128ArithmeticOP::Div => "div_i128",
            I128ArithmeticOP::Rem => "rem_i128",
        }
    }
}

#[derive(Clone, Copy)]
pub enum CsrAddress {
    Vstart = 0x8,
    Vxsat = 0x9,
    Vxrm = 0xa,
    Vcsr = 0xf,
    Vl = 0xc20,
    Vtype = 0xc21,
    Vlenb = 0xc22,
}

impl std::fmt::Debug for CsrAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "0x{:x}", self.as_u32())
    }
}

impl Display for CsrAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "0x{:x}", self.as_u32())
    }
}
impl CsrAddress {
    pub(crate) fn as_u32(self) -> u32 {
        self as u32
    }
}

pub(crate) struct VType {
    /*
        todo::I have no ida vma and vta means.
    */
    vma: bool,
    vta: bool,
    vsew: Vsew,
    valmul: Vlmul,
}

impl VType {
    fn as_u32(self) -> u32 {
        self.valmul.as_u32()
            | self.vsew.as_u32() << 3
            | if self.vta { 1 << 7 } else { 0 }
            | if self.vma { 1 << 8 } else { 0 }
    }

    const fn vill_bit() -> u64 {
        1 << 63
    }
}

enum Vlmul {
    vlmul_1_div_8 = 0b101,
    vlmul_1_div_4 = 0b110,
    vlmul_1_div_2 = 0b111,
    vlmul_1 = 0b000,
    vlmul_2 = 0b001,
    vlmul_4 = 0b010,
    vlmul_8 = 0b011,
}

impl Vlmul {
    fn as_u32(self) -> u32 {
        self as u32
    }
}

enum Vsew {
    sew_8 = 0b000,
    sew_16 = 0b001,
    sew_32 = 0b010,
    sew_64 = 0b011,
}

impl Vsew {
    fn as_u32(self) -> u32 {
        self as u32
    }
}

impl CsrOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            CsrOP::Csrrw => "csrrw",
            CsrOP::Csrrs => "csrrs",
            CsrOP::Csrrc => "csrrc",
            CsrOP::Csrrwi => "csrrwi",
            CsrOP::Csrrsi => "csrrsi",
            CsrOP::Csrrci => "csrrci",
        }
    }

    pub(crate) const fn need_rs(self) -> bool {
        match self {
            CsrOP::Csrrw | CsrOP::Csrrs | CsrOP::Csrrc => true,
            _ => false,
        }
    }
    pub(crate) const fn op_code(self) -> u32 {
        0b1110011
    }

    pub(crate) fn funct3(self) -> u32 {
        match self {
            CsrOP::Csrrw => 0b001,
            CsrOP::Csrrs => 0b010,
            CsrOP::Csrrc => 0b011,
            CsrOP::Csrrwi => 0b101,
            CsrOP::Csrrsi => 0b110,
            CsrOP::Csrrci => 0b110,
        }
    }

    pub(crate) fn rs1(self, rs: Option<Reg>, zimm: OptionUimm5) -> u32 {
        if self.need_rs() {
            reg_to_gpr_num(rs.unwrap())
        } else {
            zimm.unwrap().as_u32()
        }
    }
}

enum Vxrm {
    // round-to-nearest-up (add +0.5 LSB)
    rnu = 0b00,
    // round-to-nearest-even
    rne = 0b01,
    //round-down (truncate)
    rdn = 0b10,
    // round-to-odd (OR bits into LSB, aka "jam")
    rod = 0b11,
}

impl Vxrm {
    pub(crate) fn as_u32(self) -> u32 {
        self as u32
    }
}

pub(crate) struct Vcsr {
    xvrm: Vxrm,
    // Fixed-point accrued saturation flag
    vxsat: bool,
}

impl Vcsr {
    pub(crate) fn as_u32(self) -> u32 {
        return if self.vxsat { 1 } else { 0 } | self.xvrm.as_u32();
    }
}

static mut V_LEN: usize = 0;

/*
    V_LEN is not contant accroding to riscv document.

    Each hart supporting a vector extension de nes two parameters: 1. The maximum size in bits of a vector element that any operation can produce or consume, ELEN ≥ 8, which must be a power of 2. 2. The number of bits in a single vector register, VLEN ≥ ELEN, which must be a power of 2, and must be no greater than 216. Standard vector extensions (Section Standard Vector Extensions) and architecture pro les may set further constraints on ELEN and VLEN.

    it is ugly, but I need this global var to pass the paramter.

*/
#[inline]
pub(crate) fn set_x_len(l: usize) {
    unsafe { V_LEN = l };
}

#[inline]
pub(crate) fn get_x_len() -> usize {
    if unsafe { V_LEN } == 0 {
        panic!("V_LEN is not set.")
    }
    unsafe { V_LEN }
}
