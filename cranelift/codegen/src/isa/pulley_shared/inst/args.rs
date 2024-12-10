//! Pulley instruction arguments.

use super::*;
use crate::machinst::abi::StackAMode;
use pulley_interpreter::encode;
use pulley_interpreter::regs::Reg as _;
use std::fmt;

/// A macro for defining a newtype of `Reg` that enforces some invariant about
/// the wrapped `Reg` (such as that it is of a particular register class).
macro_rules! newtype_of_reg {
    (
        $newtype_reg:ident,
        $newtype_writable_reg:ident,
        $class:expr
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

        impl TryFrom<Reg> for $newtype_reg {
            type Error = ();
            fn try_from(r: Reg) -> Result<Self, Self::Error> {
                Self::new(r).ok_or(())
            }
        }

        impl $newtype_reg {
            /// Create this newtype from the given register, or return `None` if the register
            /// is not a valid instance of this newtype.
            pub fn new(reg: Reg) -> Option<Self> {
                if reg.class() == $class {
                    Some(Self(reg))
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

        /// If you know what you're doing, you can explicitly mutably borrow the
        /// underlying `Reg`. Don't make it point to the wrong type of register
        /// please.
        impl AsMut<Reg> for $newtype_reg {
            fn as_mut(&mut self) -> &mut Reg {
                &mut self.0
            }
        }

        /// Writable Reg.
        pub type $newtype_writable_reg = Writable<$newtype_reg>;

        impl From<pulley_interpreter::regs::$newtype_reg> for $newtype_reg {
            fn from(r: pulley_interpreter::regs::$newtype_reg) -> Self {
                Self::new(regalloc2::PReg::new(usize::from(r as u8), $class).into()).unwrap()
            }
        }
        impl From<$newtype_reg> for pulley_interpreter::regs::$newtype_reg {
            fn from(r: $newtype_reg) -> Self {
                Self::new(r.to_real_reg().unwrap().hw_enc()).unwrap()
            }
        }
        impl<'a> From<&'a $newtype_reg> for pulley_interpreter::regs::$newtype_reg {
            fn from(r: &'a $newtype_reg) -> Self {
                Self::new(r.to_real_reg().unwrap().hw_enc()).unwrap()
            }
        }
        impl From<$newtype_writable_reg> for pulley_interpreter::regs::$newtype_reg {
            fn from(r: $newtype_writable_reg) -> Self {
                Self::new(r.to_reg().to_real_reg().unwrap().hw_enc()).unwrap()
            }
        }
        impl<'a> From<&'a $newtype_writable_reg> for pulley_interpreter::regs::$newtype_reg {
            fn from(r: &'a $newtype_writable_reg) -> Self {
                Self::new(r.to_reg().to_real_reg().unwrap().hw_enc()).unwrap()
            }
        }

        impl TryFrom<Writable<Reg>> for $newtype_writable_reg {
            type Error = ();
            fn try_from(r: Writable<Reg>) -> Result<Self, Self::Error> {
                let r = r.to_reg();
                match $newtype_reg::new(r) {
                    Some(r) => Ok(Writable::from_reg(r)),
                    None => Err(()),
                }
            }
        }
    };
}

// Newtypes for registers classes.
newtype_of_reg!(XReg, WritableXReg, RegClass::Int);
newtype_of_reg!(FReg, WritableFReg, RegClass::Float);
newtype_of_reg!(VReg, WritableVReg, RegClass::Vector);

impl XReg {
    /// Index of the first "special" register, or the end of which registers
    /// regalloc is allowed to use.
    pub const SPECIAL_START: u8 = pulley_interpreter::regs::XReg::SPECIAL_START;

    /// Returns whether this is a "special" physical register for pulley.
    pub fn is_special(&self) -> bool {
        match self.as_pulley() {
            Some(reg) => reg.is_special(),
            None => false,
        }
    }

    /// Returns the pulley-typed register, if this is a phyiscal register.
    pub fn as_pulley(&self) -> Option<pulley_interpreter::XReg> {
        let enc = self.to_real_reg()?.hw_enc();
        Some(pulley_interpreter::XReg::new(enc).unwrap())
    }
}

pub use super::super::lower::isle::generated_code::ExtKind;

pub use super::super::lower::isle::generated_code::Amode;

impl Amode {
    /// Add the registers referenced by this Amode to `collector`.
    pub(crate) fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        match self {
            Amode::RegOffset { base, offset: _ } => collector.reg_use(base),
            // Registers used in these modes aren't allocatable.
            Amode::SpOffset { .. } | Amode::Stack { .. } => {}
        }
    }

    pub(crate) fn get_base_register(&self) -> Option<XReg> {
        match self {
            Amode::RegOffset { base, offset: _ } => Some((*base).into()),
            Amode::SpOffset { .. } | Amode::Stack { .. } => Some(XReg::new(stack_reg()).unwrap()),
        }
    }

    pub(crate) fn get_offset_with_state<P>(&self, state: &EmitState<P>) -> i32
    where
        P: PulleyTargetKind,
    {
        match self {
            Amode::RegOffset { base: _, offset } | Amode::SpOffset { offset } => *offset,
            Amode::Stack { amode } => {
                let offset64 = match amode {
                    StackAMode::IncomingArg(offset, stack_args_size) => {
                        let offset = i64::from(*stack_args_size) - *offset;
                        let frame_layout = state.frame_layout();
                        let sp_offset = frame_layout.tail_args_size
                            + frame_layout.setup_area_size
                            + frame_layout.clobber_size
                            + frame_layout.fixed_frame_storage_size
                            + frame_layout.outgoing_args_size;
                        i64::from(sp_offset) - offset
                    }
                    StackAMode::Slot(offset) => {
                        offset + i64::from(state.frame_layout().outgoing_args_size)
                    }
                    StackAMode::OutgoingArg(offset) => *offset,
                };
                i32::try_from(offset64).unwrap()
            }
        }
    }
}

impl core::fmt::Display for Amode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Amode::SpOffset { offset } => {
                if *offset >= 0 {
                    write!(f, "sp+{offset}")
                } else {
                    write!(f, "sp{offset}")
                }
            }
            Amode::RegOffset { base, offset } => {
                let name = reg_name(**base);
                if *offset >= 0 {
                    write!(f, "{name}+{offset}")
                } else {
                    write!(f, "{name}{offset}")
                }
            }
            Amode::Stack { amode } => core::fmt::Debug::fmt(amode, f),
        }
    }
}

impl From<StackAMode> for Amode {
    fn from(amode: StackAMode) -> Self {
        Amode::Stack { amode }
    }
}

/// The size of an operand or operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandSize {
    /// 32 bits.
    Size32,
    /// 64 bits.
    Size64,
}

pub use crate::isa::pulley_shared::lower::isle::generated_code::Cond;

impl Cond {
    /// Collect register operands within `collector` for register allocation.
    pub fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        match self {
            Cond::If32 { reg } | Cond::IfNot32 { reg } => collector.reg_use(reg),

            Cond::IfXeq32 { src1, src2 }
            | Cond::IfXneq32 { src1, src2 }
            | Cond::IfXslt32 { src1, src2 }
            | Cond::IfXslteq32 { src1, src2 }
            | Cond::IfXult32 { src1, src2 }
            | Cond::IfXulteq32 { src1, src2 }
            | Cond::IfXeq64 { src1, src2 }
            | Cond::IfXneq64 { src1, src2 }
            | Cond::IfXslt64 { src1, src2 }
            | Cond::IfXslteq64 { src1, src2 }
            | Cond::IfXult64 { src1, src2 }
            | Cond::IfXulteq64 { src1, src2 } => {
                collector.reg_use(src1);
                collector.reg_use(src2);
            }
        }
    }

    /// Encode this condition as a branch into `sink`.
    ///
    /// Note that the offset encoded to jump by is filled in as 0 and it's
    /// assumed `MachBuffer` will come back and clean it up.
    pub fn encode(&self, sink: &mut impl Extend<u8>) {
        match self {
            Cond::If32 { reg } => encode::br_if32(sink, reg, 0),
            Cond::IfNot32 { reg } => encode::br_if_not32(sink, reg, 0),
            Cond::IfXeq32 { src1, src2 } => encode::br_if_xeq32(sink, src1, src2, 0),
            Cond::IfXneq32 { src1, src2 } => encode::br_if_xneq32(sink, src1, src2, 0),
            Cond::IfXslt32 { src1, src2 } => encode::br_if_xslt32(sink, src1, src2, 0),
            Cond::IfXslteq32 { src1, src2 } => encode::br_if_xslteq32(sink, src1, src2, 0),
            Cond::IfXult32 { src1, src2 } => encode::br_if_xult32(sink, src1, src2, 0),
            Cond::IfXulteq32 { src1, src2 } => encode::br_if_xulteq32(sink, src1, src2, 0),
            Cond::IfXeq64 { src1, src2 } => encode::br_if_xeq64(sink, src1, src2, 0),
            Cond::IfXneq64 { src1, src2 } => encode::br_if_xneq64(sink, src1, src2, 0),
            Cond::IfXslt64 { src1, src2 } => encode::br_if_xslt64(sink, src1, src2, 0),
            Cond::IfXslteq64 { src1, src2 } => encode::br_if_xslteq64(sink, src1, src2, 0),
            Cond::IfXult64 { src1, src2 } => encode::br_if_xult64(sink, src1, src2, 0),
            Cond::IfXulteq64 { src1, src2 } => encode::br_if_xulteq64(sink, src1, src2, 0),
        }
    }

    /// Inverts this conditional.
    pub fn invert(&self) -> Cond {
        match *self {
            Cond::If32 { reg } => Cond::IfNot32 { reg },
            Cond::IfNot32 { reg } => Cond::If32 { reg },
            Cond::IfXeq32 { src1, src2 } => Cond::IfXneq32 { src1, src2 },
            Cond::IfXneq32 { src1, src2 } => Cond::IfXeq32 { src1, src2 },
            Cond::IfXeq64 { src1, src2 } => Cond::IfXneq64 { src1, src2 },
            Cond::IfXneq64 { src1, src2 } => Cond::IfXeq64 { src1, src2 },

            // Note that for below the condition changes but the operands are
            // also swapped.
            Cond::IfXslt32 { src1, src2 } => Cond::IfXslteq32 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXslteq32 { src1, src2 } => Cond::IfXslt32 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXult32 { src1, src2 } => Cond::IfXulteq32 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXulteq32 { src1, src2 } => Cond::IfXult32 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXslt64 { src1, src2 } => Cond::IfXslteq64 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXslteq64 { src1, src2 } => Cond::IfXslt64 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXult64 { src1, src2 } => Cond::IfXulteq64 {
                src1: src2,
                src2: src1,
            },
            Cond::IfXulteq64 { src1, src2 } => Cond::IfXult64 {
                src1: src2,
                src2: src1,
            },
        }
    }
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cond::If32 { reg } => write!(f, "if32 {}", reg_name(**reg)),
            Cond::IfNot32 { reg } => write!(f, "if_not32 {}", reg_name(**reg)),
            Cond::IfXeq32 { src1, src2 } => {
                write!(f, "if_xeq32 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXneq32 { src1, src2 } => {
                write!(f, "if_xneq32 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXslt32 { src1, src2 } => {
                write!(f, "if_xslt32 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXslteq32 { src1, src2 } => {
                write!(f, "if_xslteq32 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXult32 { src1, src2 } => {
                write!(f, "if_xult32 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXulteq32 { src1, src2 } => {
                write!(f, "if_xulteq32 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXeq64 { src1, src2 } => {
                write!(f, "if_xeq64 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXneq64 { src1, src2 } => {
                write!(f, "if_xneq64 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXslt64 { src1, src2 } => {
                write!(f, "if_xslt64 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXslteq64 { src1, src2 } => {
                write!(f, "if_xslteq64 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXult64 { src1, src2 } => {
                write!(f, "if_xult64 {}, {}", reg_name(**src1), reg_name(**src2))
            }
            Cond::IfXulteq64 { src1, src2 } => {
                write!(f, "if_xulteq64 {}, {}", reg_name(**src1), reg_name(**src2))
            }
        }
    }
}
