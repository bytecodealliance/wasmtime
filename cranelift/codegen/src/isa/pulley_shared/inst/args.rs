//! Pulley instruction arguments.

use super::*;
use crate::machinst::abi::StackAMode;
use pulley_interpreter::regs::Reg as _;

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

    pub(crate) fn get_base_register(&self) -> Option<Reg> {
        match self {
            Amode::RegOffset { base, offset: _ } => Some((*base).into()),
            Amode::SpOffset { .. } | Amode::Stack { .. } => Some(stack_reg()),
        }
    }

    pub(crate) fn get_offset_with_state<P>(&self, state: &EmitState<P>) -> i64
    where
        P: PulleyTargetKind,
    {
        match self {
            Amode::RegOffset { base: _, offset } | Amode::SpOffset { offset } => *offset,
            Amode::Stack { amode } => match amode {
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
                StackAMode::Slot(offset) => *offset + state.virtual_sp_offset,
                StackAMode::OutgoingArg(offset) => *offset,
            },
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
