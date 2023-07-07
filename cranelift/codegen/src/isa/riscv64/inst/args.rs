//! Riscv64 ISA definitions: instruction arguments.

// Some variants are never constructed, but we still want them as options in the future.
#![allow(dead_code)]
use super::*;
use crate::ir::condcodes::CondCode;

use crate::isa::riscv64::inst::{reg_name, reg_to_gpr_num};
use crate::machinst::isle::WritableReg;

use std::fmt::{Display, Formatter, Result};

/// A macro for defining a newtype of `Reg` that enforces some invariant about
/// the wrapped `Reg` (such as that it is of a particular register class).
macro_rules! newtype_of_reg {
    (
        $newtype_reg:ident,
        $newtype_writable_reg:ident,
        |$check_reg:ident| $check:expr
    ) => {
        /// A newtype wrapper around `Reg`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $newtype_reg(Reg);

        impl PartialEq<Reg> for $newtype_reg {
            fn eq(&self, other: &Reg) -> bool {
                self.0 == *other
            }
        }

        impl From<$newtype_reg> for Reg {
            fn from(r: $newtype_reg) -> Self {
                r.0
            }
        }

        impl $newtype_reg {
            /// Create this newtype from the given register, or return `None` if the register
            /// is not a valid instance of this newtype.
            pub fn new($check_reg: Reg) -> Option<Self> {
                if $check {
                    Some(Self($check_reg))
                } else {
                    None
                }
            }

            /// Get this newtype's underlying `Reg`.
            pub fn to_reg(self) -> Reg {
                self.0
            }
        }

        // Convenience impl so that people working with this newtype can use it
        // "just like" a plain `Reg`.
        //
        // NB: We cannot implement `DerefMut` because that would let people do
        // nasty stuff like `*my_xreg.deref_mut() = some_freg`, breaking the
        // invariants that `XReg` provides.
        impl std::ops::Deref for $newtype_reg {
            type Target = Reg;

            fn deref(&self) -> &Reg {
                &self.0
            }
        }

        /// Writable Reg.
        pub type $newtype_writable_reg = Writable<$newtype_reg>;
    };
}

// Newtypes for registers classes.
newtype_of_reg!(XReg, WritableXReg, |reg| reg.class() == RegClass::Int);
newtype_of_reg!(FReg, WritableFReg, |reg| reg.class() == RegClass::Float);
newtype_of_reg!(VReg, WritableVReg, |reg| reg.class() == RegClass::Vector);

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
    /// [crate::isa::riscv64::abi](the ABI module) for more details.
    NominalSPOffset(i64, Type),

    /// A reference to a constant which is placed outside of the function's
    /// body, typically at the end.
    Const(VCodeConstant),

    /// A reference to a label.
    Label(MachLabel),
}

impl AMode {
    pub(crate) fn with_allocs(self, allocs: &mut AllocationConsumer<'_>) -> Self {
        match self {
            AMode::RegOffset(reg, offset, ty) => AMode::RegOffset(allocs.next(reg), offset, ty),
            AMode::SPOffset(..)
            | AMode::FPOffset(..)
            | AMode::NominalSPOffset(..)
            | AMode::Const(..)
            | AMode::Label(..) => self,
        }
    }

    /// Returns the registers that known to the register allocator.
    /// Keep this in sync with `with_allocs`.
    pub(crate) fn get_allocatable_register(&self) -> Option<Reg> {
        match self {
            AMode::RegOffset(reg, ..) => Some(*reg),
            AMode::SPOffset(..)
            | AMode::FPOffset(..)
            | AMode::NominalSPOffset(..)
            | AMode::Const(..)
            | AMode::Label(..) => None,
        }
    }

    pub(crate) fn get_base_register(&self) -> Option<Reg> {
        match self {
            &AMode::RegOffset(reg, ..) => Some(reg),
            &AMode::SPOffset(..) => Some(stack_reg()),
            &AMode::FPOffset(..) => Some(fp_reg()),
            &AMode::NominalSPOffset(..) => Some(stack_reg()),
            &AMode::Const(..) | AMode::Label(..) => None,
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
            &AMode::Const(_) | &AMode::Label(_) => 0,
        }
    }

    pub(crate) fn to_string_with_alloc(&self, allocs: &mut AllocationConsumer<'_>) -> String {
        format!("{}", self.clone().with_allocs(allocs))
    }
}

impl Display for AMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            &AMode::RegOffset(r, offset, ..) => {
                write!(f, "{}({})", offset, reg_name(r))
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
            &AMode::Const(addr, ..) => {
                write!(f, "[const({})]", addr.as_u32())
            }
            &AMode::Label(label) => {
                write!(f, "[label{}]", label.as_u32())
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
    pub(crate) fn funct3(self) -> u32 {
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

    // funct3 and if need inverse the register
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
        }
    }

    #[inline]
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
        }
    }

    pub(crate) fn emit(self) -> u32 {
        let (funct3, reverse) = self.funct3();
        let (rs1, rs2) = if reverse {
            (self.rs2, self.rs1)
        } else {
            (self.rs1, self.rs2)
        };

        self.op_code()
            | funct3.funct3() << 12
            | reg_to_gpr_num(rs1) << 15
            | reg_to_gpr_num(rs2) << 20
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

    pub(crate) fn is_convert_to_int(self) -> bool {
        match self {
            Self::FcvtWS
            | Self::FcvtWuS
            | Self::FcvtLS
            | Self::FcvtLuS
            | Self::FcvtWD
            | Self::FcvtWuD
            | Self::FcvtLD
            | Self::FcvtLuD => true,
            _ => false,
        }
    }
    // move from x register to float register.
    pub(crate) fn move_x_to_f_op(ty: Type) -> Self {
        match ty {
            F32 => Self::FmvWX,
            F64 => Self::FmvDX,
            _ => unreachable!("ty:{:?}", ty),
        }
    }

    // move from f register to x register.
    pub(crate) fn move_f_to_x_op(ty: Type) -> Self {
        match ty {
            F32 => Self::FmvXW,
            F64 => Self::FmvXD,
            _ => unreachable!("ty:{:?}", ty),
        }
    }

    pub(crate) fn float_convert_2_int_op(from: Type, is_type_signed: bool, to: Type) -> Self {
        let type_32 = to.bits() <= 32;
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
    pub fn is_32(self) -> bool {
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
            | Self::FleS => true,
            _ => false,
        }
    }

    pub fn is_copy_sign(self) -> bool {
        match self {
            Self::FsgnjD | Self::FsgnjS => true,
            _ => false,
        }
    }

    pub fn is_copy_neg_sign(self) -> bool {
        match self {
            Self::FsgnjnD | Self::FsgnjnS => true,
            _ => false,
        }
    }
    pub fn is_copy_xor_sign(self) -> bool {
        match self {
            Self::FsgnjxS | Self::FsgnjxD => true,
            _ => false,
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
            Self::Sgt => "sgt",
            Self::SltU => "sltu",
            Self::Sgtu => "sgtu",
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
            Self::Pack => "pack",
            Self::Packw => "packw",
            Self::Packh => "packh",
        }
    }

    pub fn funct3(self) -> u32 {
        match self {
            AluOPRRR::Add => 0b000,
            AluOPRRR::Sll => 0b001,
            AluOPRRR::Slt => 0b010,
            AluOPRRR::Sgt => 0b010,
            AluOPRRR::SltU => 0b011,
            AluOPRRR::Sgtu => 0b011,
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

            // Zbb
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

            // Zbkb
            AluOPRRR::Pack => 0b100,
            AluOPRRR::Packw => 0b100,
            AluOPRRR::Packh => 0b111,
        }
    }

    pub fn op_code(self) -> u32 {
        match self {
            AluOPRRR::Add
            | AluOPRRR::Sub
            | AluOPRRR::Sll
            | AluOPRRR::Slt
            | AluOPRRR::Sgt
            | AluOPRRR::SltU
            | AluOPRRR::Sgtu
            | AluOPRRR::Xor
            | AluOPRRR::Srl
            | AluOPRRR::Sra
            | AluOPRRR::Or
            | AluOPRRR::And
            | AluOPRRR::Pack
            | AluOPRRR::Packh => 0b0110011,

            AluOPRRR::Addw
            | AluOPRRR::Subw
            | AluOPRRR::Sllw
            | AluOPRRR::Srlw
            | AluOPRRR::Sraw
            | AluOPRRR::Packw => 0b0111011,

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
            AluOPRRR::Sgt => 0b0000000,
            AluOPRRR::SltU => 0b0000000,
            AluOPRRR::Sgtu => 0b0000000,

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

            // Zbkb
            AluOPRRR::Pack => 0b0000100,
            AluOPRRR::Packw => 0b0000100,
            AluOPRRR::Packh => 0b0000100,
        }
    }

    pub(crate) fn reverse_rs(self) -> bool {
        // special case.
        // sgt and sgtu is not defined in isa.
        // emit should reverse rs1 and rs2.
        self == AluOPRRR::Sgt || self == AluOPRRR::Sgtu
    }
}

impl AluOPRRI {
    pub(crate) fn option_funct6(self) -> Option<u32> {
        let x: Option<u32> = match self {
            Self::Slli => Some(0b00_0000),
            Self::Srli => Some(0b00_0000),
            Self::Srai => Some(0b01_0000),
            Self::Bclri => Some(0b010010),
            Self::Bexti => Some(0b010010),
            Self::Binvi => Some(0b011010),
            Self::Bseti => Some(0b001010),
            Self::Rori => Some(0b011000),
            Self::SlliUw => Some(0b000010),
            _ => None,
        };
        x
    }

    pub(crate) fn option_funct7(self) -> Option<u32> {
        let x = match self {
            Self::Slliw => Some(0b000_0000),
            Self::SrliW => Some(0b000_0000),
            Self::Sraiw => Some(0b010_0000),
            Self::Roriw => Some(0b0110000),
            _ => None,
        };
        x
    }

    pub(crate) fn imm12(self, imm12: Imm12) -> u32 {
        let x = imm12.as_u32();
        if let Some(func) = self.option_funct6() {
            func << 6 | (x & 0b11_1111)
        } else if let Some(func) = self.option_funct7() {
            func << 5 | (x & 0b1_1111)
        } else if let Some(func) = self.option_funct12() {
            func
        } else {
            x
        }
    }

    pub(crate) fn option_funct12(self) -> Option<u32> {
        match self {
            Self::Clz => Some(0b011000000000),
            Self::Clzw => Some(0b011000000000),
            Self::Cpop => Some(0b011000000010),
            Self::Cpopw => Some(0b011000000010),
            Self::Ctz => Some(0b011000000001),
            Self::Ctzw => Some(0b011000000001),
            Self::Rev8 => Some(0b011010111000),
            Self::Sextb => Some(0b011000000100),
            Self::Sexth => Some(0b011000000101),
            Self::Zexth => Some(0b000010000000),
            Self::Orcb => Some(0b001010000111),
            Self::Brev8 => Some(0b0110_1000_0111),
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
}

impl Default for FRM {
    fn default() -> Self {
        Self::Fcsr
    }
}

/// float rounding mode.
impl FRM {
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

    #[inline]
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
    #[inline]
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
            R32 => Self::Lwu,
            R64 | I64 => Self::Ld,

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
    pub(crate) fn is_infinite_bits() -> u32 {
        Self::PosInfinite.bit() | Self::NegInfinite.bit()
    }
}

impl AtomicOP {
    #[inline]
    pub(crate) fn is_load(self) -> bool {
        match self {
            Self::LrW | Self::LrD => true,
            _ => false,
        }
    }

    #[inline]
    pub(crate) fn op_name(self, amo: AMO) -> String {
        let s = match self {
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
        };
        format!("{}{}", s, amo.to_static_str())
    }
    #[inline]
    pub(crate) fn op_code(self) -> u32 {
        0b0101111
    }

    #[inline]
    pub(crate) fn funct7(self, amo: AMO) -> u32 {
        self.funct5() << 2 | amo.as_u32() & 0b11
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

    pub(crate) fn load_op(t: Type) -> Self {
        if t.bits() <= 32 {
            Self::LrW
        } else {
            Self::LrD
        }
    }
    pub(crate) fn store_op(t: Type) -> Self {
        if t.bits() <= 32 {
            Self::ScW
        } else {
            Self::ScD
        }
    }

    /// extract
    pub(crate) fn extract(rd: WritableReg, offset: Reg, rs: Reg, ty: Type) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Srl,
            rd: rd,
            rs1: rs,
            rs2: offset,
        });
        //
        insts.push(Inst::Extend {
            rd: rd,
            rn: rd.to_reg(),
            signed: false,
            from_bits: ty.bits() as u8,
            to_bits: 64,
        });
        insts
    }

    /// like extract but sign extend the value.
    /// suitable for smax,etc.
    pub(crate) fn extract_sext(
        rd: WritableReg,
        offset: Reg,
        rs: Reg,
        ty: Type,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Srl,
            rd: rd,
            rs1: rs,
            rs2: offset,
        });
        //
        insts.push(Inst::Extend {
            rd: rd,
            rn: rd.to_reg(),
            signed: true,
            from_bits: ty.bits() as u8,
            to_bits: 64,
        });
        insts
    }

    pub(crate) fn unset(
        rd: WritableReg,
        tmp: WritableReg,
        offset: Reg,
        ty: Type,
    ) -> SmallInstVec<Inst> {
        assert!(rd != tmp);
        let mut insts = SmallInstVec::new();
        insts.extend(Inst::load_int_mask(tmp, ty));
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Sll,
            rd: tmp,
            rs1: tmp.to_reg(),
            rs2: offset,
        });
        insts.push(Inst::construct_bit_not(tmp, tmp.to_reg()));
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::And,
            rd: rd,
            rs1: rd.to_reg(),
            rs2: tmp.to_reg(),
        });
        insts
    }

    pub(crate) fn set(
        rd: WritableReg,
        tmp: WritableReg,
        offset: Reg,
        rs: Reg,
        ty: Type,
    ) -> SmallInstVec<Inst> {
        assert!(rd != tmp);
        let mut insts = SmallInstVec::new();
        // make rs into tmp.
        insts.push(Inst::Extend {
            rd: tmp,
            rn: rs,
            signed: false,
            from_bits: ty.bits() as u8,
            to_bits: 64,
        });
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Sll,
            rd: tmp,
            rs1: tmp.to_reg(),
            rs2: offset,
        });
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Or,
            rd: rd,
            rs1: rd.to_reg(),
            rs2: tmp.to_reg(),
        });
        insts
    }

    /// Merge reset part of rs into rd.
    /// Call this function must make sure that other part of value is already in rd.
    pub(crate) fn merge(
        rd: WritableReg,
        tmp: WritableReg,
        offset: Reg,
        rs: Reg,
        ty: Type,
    ) -> SmallInstVec<Inst> {
        let mut insts = Self::unset(rd, tmp, offset, ty);
        insts.extend(Self::set(rd, tmp, offset, rs, ty));
        insts
    }
}

impl IntSelectOP {
    #[inline]
    pub(crate) fn from_ir_op(op: crate::ir::Opcode) -> Self {
        match op {
            crate::ir::Opcode::Smax => Self::Smax,
            crate::ir::Opcode::Umax => Self::Umax,
            crate::ir::Opcode::Smin => Self::Smin,
            crate::ir::Opcode::Umin => Self::Umin,
            _ => unreachable!(),
        }
    }
    #[inline]
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            IntSelectOP::Smax => "smax",
            IntSelectOP::Umax => "umax",
            IntSelectOP::Smin => "smin",
            IntSelectOP::Umin => "umin",
        }
    }
    #[inline]
    pub(crate) fn to_int_cc(self) -> IntCC {
        match self {
            IntSelectOP::Smax => IntCC::SignedGreaterThan,
            IntSelectOP::Umax => IntCC::UnsignedGreaterThan,
            IntSelectOP::Smin => IntCC::SignedLessThan,
            IntSelectOP::Umin => IntCC::UnsignedLessThan,
        }
    }
}

///Atomic Memory ordering.
#[derive(Copy, Clone, Debug)]
pub enum AMO {
    Relax = 0b00,
    Release = 0b01,
    Aquire = 0b10,
    SeqCst = 0b11,
}

impl AMO {
    pub(crate) fn to_static_str(self) -> &'static str {
        match self {
            AMO::Relax => "",
            AMO::Release => ".rl",
            AMO::Aquire => ".aq",
            AMO::SeqCst => ".aqrl",
        }
    }
    pub(crate) fn as_u32(self) -> u32 {
        self as u32
    }
}

impl Inst {
    /// fence request bits.
    pub(crate) const FENCE_REQ_I: u8 = 1 << 3;
    pub(crate) const FENCE_REQ_O: u8 = 1 << 2;
    pub(crate) const FENCE_REQ_R: u8 = 1 << 1;
    pub(crate) const FENCE_REQ_W: u8 = 1 << 0;
    pub(crate) fn fence_req_to_string(x: u8) -> String {
        let mut s = String::default();
        if x & Self::FENCE_REQ_I != 0 {
            s.push_str("i");
        }
        if x & Self::FENCE_REQ_O != 0 {
            s.push_str("o");
        }
        if x & Self::FENCE_REQ_R != 0 {
            s.push_str("r");
        }
        if x & Self::FENCE_REQ_W != 0 {
            s.push_str("w");
        }
        s
    }
}

impl FloatRoundOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            FloatRoundOP::Nearest => "nearest",
            FloatRoundOP::Ceil => "ceil",
            FloatRoundOP::Floor => "floor",
            FloatRoundOP::Trunc => "trunc",
        }
    }

    pub(crate) fn to_frm(self) -> FRM {
        match self {
            FloatRoundOP::Nearest => FRM::RNE,
            FloatRoundOP::Ceil => FRM::RUP,
            FloatRoundOP::Floor => FRM::RDN,
            FloatRoundOP::Trunc => FRM::RTZ,
        }
    }
}

impl FloatSelectOP {
    pub(crate) fn op_name(self) -> &'static str {
        match self {
            FloatSelectOP::Max => "max",
            FloatSelectOP::Min => "min",
        }
    }

    pub(crate) fn to_fpuoprrr(self, ty: Type) -> FpuOPRRR {
        match self {
            FloatSelectOP::Max => {
                if ty == F32 {
                    FpuOPRRR::FmaxS
                } else {
                    FpuOPRRR::FmaxD
                }
            }
            FloatSelectOP::Min => {
                if ty == F32 {
                    FpuOPRRR::FminS
                } else {
                    FpuOPRRR::FminD
                }
            }
        }
    }
    // move qnan bits into int register.
    pub(crate) fn snan_bits(self, rd: Writable<Reg>, ty: Type) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        insts.push(Inst::load_imm12(rd, Imm12::from_bits(-1)));
        let x = if ty == F32 { 22 } else { 51 };
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Srli,
            rd: rd,
            rs: rd.to_reg(),
            imm12: Imm12::from_bits(x),
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Slli,
            rd: rd,
            rs: rd.to_reg(),
            imm12: Imm12::from_bits(x),
        });
        insts
    }
}

pub(crate) fn f32_bits(f: f32) -> u32 {
    u32::from_le_bytes(f.to_le_bytes())
}
pub(crate) fn f64_bits(f: f64) -> u64 {
    u64::from_le_bytes(f.to_le_bytes())
}

///
pub(crate) fn f32_cvt_to_int_bounds(signed: bool, out_bits: u8) -> (f32, f32) {
    match (signed, out_bits) {
        (true, 8) => (i8::min_value() as f32 - 1., i8::max_value() as f32 + 1.),
        (true, 16) => (i16::min_value() as f32 - 1., i16::max_value() as f32 + 1.),
        (true, 32) => (-2147483904.0, 2147483648.0),
        (true, 64) => (-9223373136366403584.0, 9223372036854775808.0),
        (false, 8) => (-1., u8::max_value() as f32 + 1.),
        (false, 16) => (-1., u16::max_value() as f32 + 1.),
        (false, 32) => (-1., 4294967296.0),
        (false, 64) => (-1., 18446744073709551616.0),
        _ => unreachable!(),
    }
}

pub(crate) fn f64_cvt_to_int_bounds(signed: bool, out_bits: u8) -> (f64, f64) {
    match (signed, out_bits) {
        (true, 8) => (i8::min_value() as f64 - 1., i8::max_value() as f64 + 1.),
        (true, 16) => (i16::min_value() as f64 - 1., i16::max_value() as f64 + 1.),
        (true, 32) => (-2147483649.0, 2147483648.0),
        (true, 64) => (-9223372036854777856.0, 9223372036854775808.0),
        (false, 8) => (-1., u8::max_value() as f64 + 1.),
        (false, 16) => (-1., u16::max_value() as f64 + 1.),
        (false, 32) => (-1., 4294967296.0),
        (false, 64) => (-1., 18446744073709551616.0),
        _ => unreachable!(),
    }
}
