//! AArch64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]

use crate::ir::condcodes::{CondCode, FloatCC};

use super::*;

pub static WORD_SIZE: u8 = 8;

use crate::machinst::*;

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
            BranchFunct3::Geu => 0b110,
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

        self.op_code()
            | funct3.bits() << 12
            | (rs1.to_real_reg().unwrap().hw_enc() as u32) << 15
            | (rs2.to_real_reg().unwrap().hw_enc() as u32) << 20
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
        //todo look like all undefined, all zero
        0
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
            AluOPRR::FsqrtS => RM,
            AluOPRR::FcvtWS => RM,
            AluOPRR::FcvtWuS => RM,
            AluOPRR::FmvXW => 0b000,
            AluOPRR::FclassS => 0b001,
            AluOPRR::FcvtSw => RM,
            AluOPRR::FcvtSwU => RM,
            AluOPRR::FmvWX => 0b000,

            AluOPRR::FcvtLS => RM,
            AluOPRR::FcvtLuS => RM,
            AluOPRR::FcvtSL => RM,
            AluOPRR::FcvtSLU => RM,

            AluOPRR::FcvtLd => RM,
            AluOPRR::FcvtLuD => RM,
            AluOPRR::FmvXD => 0b000,
            AluOPRR::FcvtDL => RM,
            AluOPRR::FcvtDLu => RM,
            AluOPRR::FmvDX => 0b000,
            AluOPRR::FcvtSd => RM,
            AluOPRR::FcvtDS => RM,
            AluOPRR::FclassD => 0b001,
            AluOPRR::FcvtWD => RM,
            AluOPRR::FcvtWuD => RM,
            AluOPRR::FcvtDW => RM,
            AluOPRR::FcvtDWU => RM,
            AluOPRR::FsqrtD => RM,
        }
    }
}

impl AluOPRRR {
    pub(crate) fn op_name(self) -> &'static str {
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
            Self::Mulhsu => "Mulhsu",
            Self::Mulhu => "Mulhu",
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

            AluOPRRR::FaddS => RM,
            AluOPRRR::FsubS => RM,
            AluOPRRR::FmulS => RM,
            AluOPRRR::FdivS => RM,

            AluOPRRR::FsgnjS => 0b000,
            AluOPRRR::FsgnjnS => 0b001,
            AluOPRRR::FsgnjxS => 0b010,
            AluOPRRR::FminS => 0b000,
            AluOPRRR::FmaxS => 0b001,

            AluOPRRR::FeqS => 0b010,
            AluOPRRR::FltS => 0b001,
            AluOPRRR::FleS => 0b000,

            AluOPRRR::FaddD => RM,
            AluOPRRR::FsubD => RM,
            AluOPRRR::FmulD => RM,
            AluOPRRR::FdivD => RM,

            AluOPRRR::FsgnjD => 0b000,
            AluOPRRR::FsgnjnD => 0b001,
            AluOPRRR::FsgnjxD => 0b010,
            AluOPRRR::FminD => 0b000,
            AluOPRRR::FmaxD => 0b001,
            AluOPRRR::FeqD => 0b010,
            AluOPRRR::FltD => 0b001,
            AluOPRRR::FleD => 0b001,
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
        }
    }

    pub fn funct7(self) -> u32 {
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
            | AluOPRRI::Srai => 0b0010011,
            AluOPRRI::Addiw | AluOPRRI::Slliw | AluOPRRI::SrliW | AluOPRRI::Sraiw => 0b0011011,
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
    // give me the option reg
    // pub(crate) fn rs1(self, reg: OptionReg) -> u32 {
    //     // current all zero
    //     if let Some(r) = reg {
    //         r.to_real_reg().unwrap().hw_enc() as u32
    //     } else {
    //         0
    //     }
    // }
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
            Self::Fsflagsi => "Fsflagsi",
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
    pub(crate) fn just_eq(&self) -> bool {
        match self {
            Self::C(_) => self.test(Self::EQ) && self.clone().clean(Self::EQ).is_zero(),
            _ => false,
        }
    }
    pub(crate) fn just_lt(&self) -> bool {
        match self {
            Self::C(_) => self.test(Self::LT) && self.clone().clean(Self::LT).is_zero(),
            _ => false,
        }
    }

    pub(crate) fn just_le(&self) -> bool {
        match self {
            Self::C(_) => {
                (self.test(Self::LT) && self.test(Self::EQ))
                    && self.clone().clean(Self::LT).clean(Self::EQ).is_zero()
            }
            _ => false,
        }
    }

    fn clean(mut self, o: Self) -> Self {
        match self {
            Self::C(ref mut x) => *x = *x & !(o.bit()),
            _ => unreachable!(),
        }
        self
    }

    fn is_zero(&self) -> bool {
        match self {
            Self::C(x) => *x == 0,
            _ => false,
        }
    }
}

impl AtomicOP {
    pub(crate) fn is_load(self) -> bool {
        match self {
            Self::LrW | Self::ScW => true,
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
    pub(crate) fn op_code(self) -> u32 {
        0b0101111
    }
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

    pub(crate) fn from_atomicrmw_type_and_op(ty: Type, op: crate::ir::AtomicRmwOp) -> Self {
        let type_32 = ty.bits() == 32;
        match op {
            crate::ir::AtomicRmwOp::Add => {
                if type_32 {
                    Self::AmoaddW
                } else {
                    Self::AmoaddD
                }
            }
            crate::ir::AtomicRmwOp::Sub => {
                if type_32 {
                    Self::AmoaddW
                } else {
                    Self::AmoaddD
                }
            }
            crate::ir::AtomicRmwOp::And => {
                if type_32 {
                    Self::AmoandW
                } else {
                    Self::AmoandD
                }
            }
            crate::ir::AtomicRmwOp::Nand => {
                if type_32 {
                    Self::AmoorW
                } else {
                    Self::AmoorD
                }
            }
            crate::ir::AtomicRmwOp::Or => {
                if type_32 {
                    Self::AmoorW
                } else {
                    Self::AmoorD
                }
            }
            crate::ir::AtomicRmwOp::Xor => {
                if type_32 {
                    Self::AmoxorW
                } else {
                    Self::AmoxorD
                }
            }
            crate::ir::AtomicRmwOp::Xchg => {
                if type_32 {
                    Self::AmoswapW
                } else {
                    Self::AmoswapD
                }
            }
            crate::ir::AtomicRmwOp::Umin => {
                if type_32 {
                    Self::AmominuW
                } else {
                    Self::AmominuD
                }
            }
            crate::ir::AtomicRmwOp::Umax => {
                if type_32 {
                    Self::AmomaxuW
                } else {
                    Self::AmomaxuD
                }
            }
            crate::ir::AtomicRmwOp::Smin => {
                if type_32 {
                    Self::AmominW
                } else {
                    Self::AmominD
                }
            }
            crate::ir::AtomicRmwOp::Smax => {
                if type_32 {
                    Self::AmomaxW
                } else {
                    Self::AmomaxD
                }
            }
        }
    }
}

impl ExtendOp {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            ExtendOp::UXTB => "uxtb",
            ExtendOp::UXTH => "uxth",
            ExtendOp::UXTW => "uxtw",
            ExtendOp::UXTD => "uxtd",
            ExtendOp::SXTB => "sxtb",
            ExtendOp::SXTH => "sxth",
            ExtendOp::SXTW => "sxtw",
            ExtendOp::SXTD => "sxtd",
        }
    }

    pub(crate) fn from_extend_args(signed: bool, from_bits: u8, to_bits: u8) -> Option<Self> {
        // match (signed, from_bits, to_bits) {
        //     (false, 1, 8) => Some(Self::UXTB),
        //     (false, _, 16) => Some(Self::UXTH),
        //     (false, _, 32) => Some(Self::UXTW),
        //     (false, _, 64) => Some(Self::UXTD),
        //     (true, 1, 8) => Some(Self::SXTB),
        //     (true, _, 16) => Some(Self::SXTH),
        //     (true, _, 32) => Some(Self::SXTW),
        //     (true, _, 64) => Some(Self::SXTD),
        //     _ => None,
        // }
        // None
        unimplemented!("not in use")
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
    unreachable!("ir {} conflict with risc-v integer flag", op)
}
#[inline(always)]
pub(crate) fn ir_fflags_conflict(op: crate::ir::Opcode) {
    unreachable!("ir {} conflict with risc-v float flag", op)
}
