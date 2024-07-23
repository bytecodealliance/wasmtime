//! Pulley registers.

use core::{fmt, ops::Range};

macro_rules! define_registers {
    (
        $(
            $( #[$attr:meta] )*
            pub struct $name:ident = $range:expr;
        )*
) => {
        $(
            $( #[ $attr ] )*
            #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(u8);

            impl fmt::Debug for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
                    fmt::Display::fmt(self, f)
                }
            }

            impl $name {
                /// The valid register range for this register class.
                pub const RANGE: Range<u8> = $range;

                /// Construct a new register of this class.
                #[inline]
                pub fn new(index: u8) -> Option<Self> {
                    if Self::RANGE.start <= index && index < Self::RANGE.end {
                        Some(unsafe { Self::unchecked_new(index) })
                    } else {
                        None
                    }
                }

                /// Construct a new register of this class without checking that
                /// `index` is a valid register index.
                #[inline]
                pub unsafe fn unchecked_new(index: u8) -> Self {
                    debug_assert!(Self::RANGE.start <= index && index < Self::RANGE.end);
                    Self(index)
                }

                /// Get this register's index.
                #[inline]
                pub fn index(&self) -> usize {
                    usize::from(self.0)
                }
            }

            #[cfg(feature = "arbitrary")]
            impl<'a> arbitrary::Arbitrary<'a> for $name {
                fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
                    let index = u.int_in_range(Self::RANGE.start..=Self::RANGE.end - 1)?;
                    Ok(Self(index))
                }
            }
        )*
    }
}

define_registers! {
    /// An `x` register: integers.
    pub struct XReg = 0..37;

    /// An `f` register: floats.
    pub struct FReg = 0..32;

    /// A `v` register: vectors.
    pub struct VReg = 0..32;
}

/// Any register, regardless of class.
///
/// Never appears inside an instruction -- instructions always name a particular
/// class of register -- but this is useful for testing and things like that.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnyReg {
    X(XReg),
    F(FReg),
    V(VReg),
}

impl From<XReg> for AnyReg {
    fn from(x: XReg) -> Self {
        Self::X(x)
    }
}

impl From<FReg> for AnyReg {
    fn from(f: FReg) -> Self {
        Self::F(f)
    }
}

impl From<VReg> for AnyReg {
    fn from(v: VReg) -> Self {
        Self::V(v)
    }
}

impl fmt::Display for AnyReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyReg::X(r) => fmt::Display::fmt(r, f),
            AnyReg::F(r) => fmt::Display::fmt(r, f),
            AnyReg::V(r) => fmt::Display::fmt(r, f),
        }
    }
}

impl fmt::Debug for AnyReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for AnyReg {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        match u.int_in_range(0..=2)? {
            0 => Ok(AnyReg::X(u.arbitrary()?)),
            1 => Ok(AnyReg::F(u.arbitrary()?)),
            2 => Ok(AnyReg::V(u.arbitrary()?)),
            _ => unreachable!(),
        }
    }
}

impl XReg {
    /// The valid special register range.
    pub const SPECIAL_RANGE: Range<u8> = 32..37;

    /// The special `sp` stack pointer register.
    pub const SP: Self = Self(32);

    /// The special `lr` link register.
    pub const LR: Self = Self(33);

    /// The special `fp` frame pointer register.
    pub const FP: Self = Self(34);

    /// The special `spilltmp0` scratch register.
    pub const SPILL_TMP_0: Self = Self(35);

    /// The special `spilltmp1` scratch register.
    pub const SPILL_TMP_1: Self = Self(36);

    /// Is this `x` register a special register?
    pub fn is_special(&self) -> bool {
        self.0 >= Self::SPECIAL_RANGE.start
    }
}

impl fmt::Display for XReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            x if *x == Self::SP => write!(f, "sp"),
            x if *x == Self::LR => write!(f, "lr"),
            x if *x == Self::FP => write!(f, "fp"),
            x if *x == Self::SPILL_TMP_0 => write!(f, "spilltmp0"),
            x if *x == Self::SPILL_TMP_1 => write!(f, "spilltmp1"),
            Self(x) => write!(f, "x{x}"),
        }
    }
}

impl fmt::Display for FReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "f{}", self.0)
    }
}

impl fmt::Display for VReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}
