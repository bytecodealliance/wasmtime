//! Riscv64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]
use super::*;
use crate::ir::condcodes::{CondCode, FloatCC};

use crate::isa::riscv64::inst::reg_to_gpr_num;

use std::fmt::{Display, Formatter, Result};

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

    pub(crate) fn to_string_with_alloc(&self, allocs: &mut AllocationConsumer<'_>) -> String {
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

/// risc-v always take two register to compare
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
    pub(crate) fn op_name(&self) -> &'static str {
        match self.kind {
            IntCC::Equal => "beq",
            IntCC::NotEqual => "bne",
            IntCC::SignedLessThan => "blt",
            IntCC::SignedGreaterThanOrEqual => "bge",
            IntCC::SignedGreaterThan => "bgt",
            IntCC::SignedLessThanOrEqual => "ble",
            IntCC::UnsignedLessThan => "bltu",
            IntCC::UnsignedGreaterThanOrEqual => "bgeu",
            IntCC::UnsignedGreaterThan => "bgtu",
            IntCC::UnsignedLessThanOrEqual => "bleu",
            IntCC::Overflow => todo!(),
            IntCC::NotOverflow => todo!(),
        }
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

impl FpuOPRRRR {
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
            FpuOPRRRR::FmaddS | FpuOPRRRR::FmsubS | FpuOPRRRR::FnmsubS | FpuOPRRRR::FnmaddS => 0,
            FpuOPRRRR::FmaddD | FpuOPRRRR::FmsubD | FpuOPRRRR::FnmsubD | FpuOPRRRR::FnmaddD => 1,
        }
    }

    pub(crate) fn funct3(self, rounding_mode: Option<FRM>) -> u32 {
        rounding_mode.unwrap_or_default().as_u32()
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            FpuOPRRRR::FmaddS => 0b1000011,
            FpuOPRRRR::FmsubS => 0b1000111,
            FpuOPRRRR::FnmsubS => 0b1001011,
            FpuOPRRRR::FnmaddS => 0b1001111,
            FpuOPRRRR::FmaddD => 0b1000011,
            FpuOPRRRR::FmsubD => 0b1000111,
            FpuOPRRRR::FnmsubD => 0b1001011,
            FpuOPRRRR::FnmaddD => 0b1001111,
        }
    }
}

impl FpuOPRR {
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
            Self::FcvtLD => "fcvt.l.d",
            Self::FcvtLuD => "fcvt.lu.d",
            Self::FmvXD => "fmv.x.d",
            Self::FcvtDL => "fcvt.d.l",
            Self::FcvtDLu => "fcvt.d.lu",
            Self::FmvDX => "fmv.d.x",
            Self::FsqrtD => "fsqrt.d",
            Self::FcvtSD => "fcvt.s.d",
            Self::FcvtDS => "fcvt.d.s",
            Self::FclassD => "fclass.d",
            Self::FcvtWD => "fcvt.w.d",
            Self::FcvtWuD => "fcvt.wu.d",
            Self::FcvtDW => "fcvt.d.w",
            Self::FcvtDWU => "fcvt.d.wu",
        }
    }

    /*
        move from x register to float register.
    */
    pub(crate) const fn move_x_to_f_op(ty: Type) -> Self {
        match ty {
            F32 => Self::FmvWX,
            F64 => Self::FmvDX,
            _ => unreachable!(),
        }
    }
    /*
        move from f register to x register.
    */
    pub(crate) const fn move_f_to_x_op(ty: Type) -> Self {
        match ty {
            F32 => Self::FmvXW,
            F64 => Self::FmvXD,
            _ => unreachable!(),
        }
    }

    pub(crate) fn float_convert_2_int_op(from: Type, is_type_signed: bool, to: Type) -> Self {
        let type_32 = to.bits() == 32;
        match from {
            F32 => {
                if is_type_signed {
                    if type_32 {
                        Self::FcvtWS
                    } else {
                        Self::FcvtLS
                    }
                } else {
                    if type_32 {
                        Self::FcvtWuS
                    } else {
                        Self::FcvtLuS
                    }
                }
            }
            F64 => {
                if is_type_signed {
                    if type_32 {
                        Self::FcvtWD
                    } else {
                        Self::FcvtLD
                    }
                } else {
                    if type_32 {
                        Self::FcvtWuD
                    } else {
                        Self::FcvtLuD
                    }
                }
            }
            _ => unreachable!("from type:{}", from),
        }
    }

    pub(crate) fn int_convert_2_float_op(from: Type, is_type_signed: bool, to: Type) -> Self {
        let type_32 = from.bits() == 32;
        match to {
            F32 => {
                if is_type_signed {
                    if type_32 {
                        Self::FcvtSw
                    } else {
                        Self::FcvtSL
                    }
                } else {
                    if type_32 {
                        Self::FcvtSwU
                    } else {
                        Self::FcvtSLU
                    }
                }
            }
            F64 => {
                if is_type_signed {
                    if type_32 {
                        Self::FcvtDW
                    } else {
                        Self::FcvtDL
                    }
                } else {
                    if type_32 {
                        Self::FcvtDWU
                    } else {
                        Self::FcvtDLu
                    }
                }
            }
            _ => unreachable!("to type:{}", to),
        }
    }

    pub(crate) fn op_code(self) -> u32 {
        match self {
            FpuOPRR::FsqrtS
            | FpuOPRR::FcvtWS
            | FpuOPRR::FcvtWuS
            | FpuOPRR::FmvXW
            | FpuOPRR::FclassS
            | FpuOPRR::FcvtSw
            | FpuOPRR::FcvtSwU
            | FpuOPRR::FmvWX => 0b1010011,

            FpuOPRR::FcvtLS | FpuOPRR::FcvtLuS | FpuOPRR::FcvtSL | FpuOPRR::FcvtSLU => 0b1010011,

            FpuOPRR::FcvtLD
            | FpuOPRR::FcvtLuD
            | FpuOPRR::FmvXD
            | FpuOPRR::FcvtDL
            | FpuOPRR::FcvtDLu
            | FpuOPRR::FmvDX => 0b1010011,

            FpuOPRR::FsqrtD
            | FpuOPRR::FcvtSD
            | FpuOPRR::FcvtDS
            | FpuOPRR::FclassD
            | FpuOPRR::FcvtWD
            | FpuOPRR::FcvtWuD
            | FpuOPRR::FcvtDW
            | FpuOPRR::FcvtDWU => 0b1010011,
        }
    }

    pub(crate) fn rs2_funct5(self) -> u32 {
        match self {
            FpuOPRR::FsqrtS => 0b00000,
            FpuOPRR::FcvtWS => 0b00000,
            FpuOPRR::FcvtWuS => 0b00001,
            FpuOPRR::FmvXW => 0b00000,
            FpuOPRR::FclassS => 0b00000,
            FpuOPRR::FcvtSw => 0b00000,
            FpuOPRR::FcvtSwU => 0b00001,
            FpuOPRR::FmvWX => 0b00000,
            FpuOPRR::FcvtLS => 0b00010,
            FpuOPRR::FcvtLuS => 0b00011,
            FpuOPRR::FcvtSL => 0b00010,
            FpuOPRR::FcvtSLU => 0b00011,
            FpuOPRR::FcvtLD => 0b00010,
            FpuOPRR::FcvtLuD => 0b00011,
            FpuOPRR::FmvXD => 0b00000,
            FpuOPRR::FcvtDL => 0b00010,
            FpuOPRR::FcvtDLu => 0b00011,
            FpuOPRR::FmvDX => 0b00000,
            FpuOPRR::FcvtSD => 0b00001,
            FpuOPRR::FcvtDS => 0b00000,
            FpuOPRR::FclassD => 0b00000,
            FpuOPRR::FcvtWD => 0b00000,
            FpuOPRR::FcvtWuD => 0b00001,
            FpuOPRR::FcvtDW => 0b00000,
            FpuOPRR::FcvtDWU => 0b00001,
            FpuOPRR::FsqrtD => 0b00000,
        }
    }
    pub(crate) fn funct7(self) -> u32 {
        match self {
            FpuOPRR::FsqrtS => 0b0101100,
            FpuOPRR::FcvtWS => 0b1100000,
            FpuOPRR::FcvtWuS => 0b1100000,
            FpuOPRR::FmvXW => 0b1110000,
            FpuOPRR::FclassS => 0b1110000,
            FpuOPRR::FcvtSw => 0b1101000,
            FpuOPRR::FcvtSwU => 0b1101000,
            FpuOPRR::FmvWX => 0b1111000,
            FpuOPRR::FcvtLS => 0b1100000,
            FpuOPRR::FcvtLuS => 0b1100000,
            FpuOPRR::FcvtSL => 0b1101000,
            FpuOPRR::FcvtSLU => 0b1101000,
            FpuOPRR::FcvtLD => 0b1100001,
            FpuOPRR::FcvtLuD => 0b1100001,
            FpuOPRR::FmvXD => 0b1110001,
            FpuOPRR::FcvtDL => 0b1101001,
            FpuOPRR::FcvtDLu => 0b1101001,
            FpuOPRR::FmvDX => 0b1111001,
            FpuOPRR::FcvtSD => 0b0100000,
            FpuOPRR::FcvtDS => 0b0100001,
            FpuOPRR::FclassD => 0b1110001,
            FpuOPRR::FcvtWD => 0b1100001,
            FpuOPRR::FcvtWuD => 0b1100001,
            FpuOPRR::FcvtDW => 0b1101001,
            FpuOPRR::FcvtDWU => 0b1101001,
            FpuOPRR::FsqrtD => 0b0101101,
        }
    }

    pub(crate) fn funct3(self, rounding_mode: Option<FRM>) -> u32 {
        let rounding_mode = rounding_mode.unwrap_or_default().as_u32();
        match self {
            FpuOPRR::FsqrtS => rounding_mode,
            FpuOPRR::FcvtWS => rounding_mode,
            FpuOPRR::FcvtWuS => rounding_mode,
            FpuOPRR::FmvXW => 0b000,
            FpuOPRR::FclassS => 0b001,
            FpuOPRR::FcvtSw => rounding_mode,
            FpuOPRR::FcvtSwU => rounding_mode,
            FpuOPRR::FmvWX => 0b000,
            FpuOPRR::FcvtLS => rounding_mode,
            FpuOPRR::FcvtLuS => rounding_mode,
            FpuOPRR::FcvtSL => rounding_mode,
            FpuOPRR::FcvtSLU => rounding_mode,
            FpuOPRR::FcvtLD => rounding_mode,
            FpuOPRR::FcvtLuD => rounding_mode,
            FpuOPRR::FmvXD => 0b000,
            FpuOPRR::FcvtDL => rounding_mode,
            FpuOPRR::FcvtDLu => rounding_mode,
            FpuOPRR::FmvDX => 0b000,
            FpuOPRR::FcvtSD => rounding_mode,
            FpuOPRR::FcvtDS => rounding_mode,
            FpuOPRR::FclassD => 0b001,
            FpuOPRR::FcvtWD => rounding_mode,
            FpuOPRR::FcvtWuD => rounding_mode,
            FpuOPRR::FcvtDW => rounding_mode,
            FpuOPRR::FcvtDWU => 0b000,
            FpuOPRR::FsqrtD => rounding_mode,
        }
    }
}

impl FpuOPRRR {
    pub(crate) const fn op_name(self) -> &'static str {
        match self {
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

    pub fn funct3(self, rounding_mode: Option<FRM>) -> u32 {
        let rounding_mode = rounding_mode.unwrap_or_default();
        let rounding_mode = rounding_mode.as_u32();
        match self {
            Self::FaddS => rounding_mode,
            Self::FsubS => rounding_mode,
            Self::FmulS => rounding_mode,
            Self::FdivS => rounding_mode,

            Self::FsgnjS => 0b000,
            Self::FsgnjnS => 0b001,
            Self::FsgnjxS => 0b010,
            Self::FminS => 0b000,
            Self::FmaxS => 0b001,

            Self::FeqS => 0b010,
            Self::FltS => 0b001,
            Self::FleS => 0b000,

            Self::FaddD => rounding_mode,
            Self::FsubD => rounding_mode,
            Self::FmulD => rounding_mode,
            Self::FdivD => rounding_mode,

            Self::FsgnjD => 0b000,
            Self::FsgnjnD => 0b001,
            Self::FsgnjxD => 0b010,
            Self::FminD => 0b000,
            Self::FmaxD => 0b001,
            Self::FeqD => 0b010,
            Self::FltD => 0b001,
            Self::FleD => 0b000,
        }
    }

    pub fn op_code(self) -> u32 {
        match self {
            Self::FaddS
            | Self::FsubS
            | Self::FmulS
            | Self::FdivS
            | Self::FsgnjS
            | Self::FsgnjnS
            | Self::FsgnjxS
            | Self::FminS
            | Self::FmaxS
            | Self::FeqS
            | Self::FltS
            | Self::FleS => 0b1010011,

            Self::FaddD
            | Self::FsubD
            | Self::FmulD
            | Self::FdivD
            | Self::FsgnjD
            | Self::FsgnjnD
            | Self::FsgnjxD
            | Self::FminD
            | Self::FmaxD
            | Self::FeqD
            | Self::FltD
            | Self::FleD => 0b1010011,
        }
    }

    pub const fn funct7(self) -> u32 {
        match self {
            Self::FaddS => 0b0000000,
            Self::FsubS => 0b0000100,
            Self::FmulS => 0b0001000,
            Self::FdivS => 0b0001100,

            Self::FsgnjS => 0b0010000,
            Self::FsgnjnS => 0b0010000,
            Self::FsgnjxS => 0b0010000,
            Self::FminS => 0b0010100,
            Self::FmaxS => 0b0010100,
            Self::FeqS => 0b1010000,
            Self::FltS => 0b1010000,
            Self::FleS => 0b1010000,
            Self::FaddD => 0b0000001,
            Self::FsubD => 0b0000101,
            Self::FmulD => 0b0001001,
            Self::FdivD => 0b0001101,

            Self::FsgnjD => 0b0010001,
            Self::FsgnjnD => 0b0010001,
            Self::FsgnjxD => 0b0010001,
            Self::FminD => 0b0010101,
            Self::FmaxD => 0b0010101,
            Self::FeqD => 0b1010001,
            Self::FltD => 0b1010001,
            Self::FleD => 0b1010001,
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
            Self::Brev8 => "brev8",
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
            AluOPRRI::Brev8 => 0b101,
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
            | AluOPRRI::Orcb
            | AluOPRRI::Brev8 => 0b0010011,

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
            | Self::Orcb
            | Self::Brev8 => true,
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

    pub(crate) fn shamt_mask(self) -> u8 {
        let s = self.need_shamt();
        match s {
            Some(x) => match x {
                5 => 0b1_1111,
                6 => 0b11_1111,
                _ => unreachable!(),
            },
            None => 0,
        }
    }

    /*
        some instruction use imm12 for function code.
        return Self and Imm12
    */
    pub(crate) fn funct12(self, shamt: Option<u8>) -> (Self, Imm12) {
        if self.need_shamt().is_some() {
            assert!(shamt.is_some());
        } else {
            assert!(shamt.is_none());
        }
        let mut shamt = shamt.map(|s| s as u32);
        let bits: u32 = match self {
            Self::Bclri => shamt.take().unwrap() | 0b010010 << 6,
            Self::Bexti => shamt.take().unwrap() | 0b010010 << 6,
            Self::Binvi => shamt.take().unwrap() | 0b011010 << 6,
            Self::Bseti => shamt.take().unwrap() | 0b001010 << 6,
            Self::Rori => shamt.take().unwrap() | 0b011000 << 6,
            Self::Roriw => shamt.take().unwrap() | 0b0110000 << 5,
            Self::SlliUw => shamt.take().unwrap() | 0b000010 << 6,
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
            Self::Brev8 => 0b0110_1000_0111,
            _ => unreachable!(),
        };
        /*
            make sure shamt is been consumed.
        */
        assert!(shamt.is_none());
        (self, Imm12::from_bits(bits as i16))
    }
}

impl Default for FRM {
    fn default() -> Self {
        Self::Fcsr
    }
}

/*
    float rounding mode.
*/
impl FRM {
    pub(crate) fn is_none_or_using_fcsr(x: Option<FRM>) -> bool {
        match x {
            Some(x) => x == FRM::Fcsr,
            None => true,
        }
    }

    pub(crate) fn to_static_str(self) -> &'static str {
        match self {
            FRM::RNE => "rne",
            FRM::RTZ => "rtz",
            FRM::RDN => "rdn",
            FRM::RUP => "rup",
            FRM::RMM => "rmm",
            FRM::Fcsr => "fcsr",
        }
    }

    #[inline(always)]
    pub(crate) fn bits(self) -> u8 {
        match self {
            FRM::RNE => 0b000,
            FRM::RTZ => 0b001,
            FRM::RDN => 0b010,
            FRM::RUP => 0b011,
            FRM::RMM => 0b100,
            FRM::Fcsr => 0b111,
        }
    }
    pub(crate) fn as_u32(self) -> u32 {
        self.bits() as u32
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

impl FClassResult {
    pub(crate) const fn bit(self) -> u32 {
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

    #[inline]
    pub(crate) const fn is_nan_bits() -> u32 {
        Self::SNaN.bit() | Self::QNaN.bit()
    }
    #[inline]
    pub(crate) fn is_zero_bits() -> u32 {
        Self::NegZero.bit() | Self::PosZero.bit()
    }

    #[inline]
    #[allow(dead_code)]
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
#[derive(Clone, Copy)]
pub enum FloatCCBit {
    UN,
    EQ,
    LT,
    GT,
    CompareSet(u8),
}

impl FloatCCBit {
    pub(crate) const fn bits(&self) -> u8 {
        match self {
            FloatCCBit::UN => 1 << 0,
            FloatCCBit::EQ => 1 << 1,
            FloatCCBit::LT => 1 << 2,
            FloatCCBit::GT => 1 << 3,
            FloatCCBit::CompareSet(x) => *x,
        }
    }

    /*
        mask bit for floatcc
    */
    pub(crate) fn floatcc_2_mask_bits<T: Into<FloatCC>>(t: T) -> Self {
        let v = match t.into() {
            FloatCC::Ordered => Self::EQ.bits() | Self::LT.bits() | Self::GT.bits(),
            FloatCC::Unordered => Self::UN.bits(),
            FloatCC::Equal => Self::EQ.bits(),
            FloatCC::NotEqual => Self::UN.bits() | Self::LT.bits() | Self::GT.bits(),
            FloatCC::OrderedNotEqual => Self::LT.bits() | Self::GT.bits(),
            FloatCC::UnorderedOrEqual => Self::UN.bits() | Self::EQ.bits(),
            FloatCC::LessThan => Self::LT.bits(),
            FloatCC::LessThanOrEqual => Self::LT.bits() | Self::EQ.bits(),
            FloatCC::GreaterThan => Self::GT.bits(),
            FloatCC::GreaterThanOrEqual => Self::GT.bits() | Self::EQ.bits(),
            FloatCC::UnorderedOrLessThan => Self::UN.bits() | Self::LT.bits(),
            FloatCC::UnorderedOrLessThanOrEqual => {
                Self::UN.bits() | Self::LT.bits() | Self::EQ.bits()
            }
            FloatCC::UnorderedOrGreaterThan => Self::UN.bits() | Self::GT.bits(),
            FloatCC::UnorderedOrGreaterThanOrEqual => {
                Self::UN.bits() | Self::GT.bits() | Self::EQ.bits()
            }
        };
        Self::CompareSet(v)
    }

    #[inline]
    pub(crate) fn has(&self, o: Self) -> bool {
        (self.bits() & o.bits()) == o.bits()
    }

    pub(crate) fn has_and_clear(&mut self, other: Self) -> bool {
        if !self.has(other) {
            return false;
        }
        self.clear_bits(other);
        return true;
    }

    #[inline]
    fn clear_bits(&mut self, c: Self) {
        match self {
            Self::CompareSet(ref mut x) => *x = *x & !c.bits(),
            _ => unreachable!(),
        }
    }
}

impl std::ops::BitOr for FloatCCBit {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::CompareSet(self.bits() | rhs.bits())
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
    // #[inline(always)]
    // pub(crate) fn is_store(self) -> bool {
    //     match self {
    //         Self::ScW | Self::ScD => true,
    //         _ => false,
    //     }
    // }
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
    pub(crate) fn funct7(self, amo: AMO) -> u32 {
        self.funct5() << 2 | amo.as_u32()
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

impl ReferenceCheckOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            ReferenceCheckOP::IsNull => "is_null",
            ReferenceCheckOP::IsInvalid => "is_invalid",
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

#[derive(Clone, Copy)]
pub enum CsrAddress {
    Fcsr = 0x3,
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

///Atomic Memory ordering.
#[derive(Copy, Clone, Debug)]
pub enum AMO {
    Relax = 0b00,
    Release = 0b01,
    Aquire = 0b10,
    SeqConsistent = 0b11,
}

impl AMO {
    pub(crate) fn to_static_str(self) -> &'static str {
        match self {
            AMO::Relax => "",
            AMO::Release => ".rl",
            AMO::Aquire => ".aq",
            AMO::SeqConsistent => ".aqrl",
        }
    }
    pub(crate) fn as_u32(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod test {
    use super::FloatCCBit;
    #[test]
    fn float_cc_bit_clear() {
        let mut x = FloatCCBit::UN | FloatCCBit::GT | FloatCCBit::EQ;
        assert!(x.has_and_clear(FloatCCBit::UN | FloatCCBit::GT));
        assert!(x.has(FloatCCBit::EQ));
        assert!(!x.has(FloatCCBit::UN));
        assert!(!x.has(FloatCCBit::GT));
    }
    #[test]
    fn float_cc_bit_has() {
        let x = FloatCCBit::UN | FloatCCBit::GT | FloatCCBit::EQ;
        assert!(x.has(FloatCCBit::UN | FloatCCBit::GT));
        assert!(!x.has(FloatCCBit::LT));
    }
}
