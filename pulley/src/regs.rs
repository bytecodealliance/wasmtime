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
    pub struct XReg = 0..32;

    /// An `f` register: floats.
    pub struct FReg = 0..32;

    /// A `v` register: vectors.
    pub struct VReg = 0..32;
}

/// An `s` register: integers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum SReg {
    /// The special `sp` stack pointer register.
    SP,

    /// The special `lr` link register.
    LR,

    /// The special `fp` frame pointer register.
    FP,

    /// The special `spilltmp0` scratch register.
    SPILL_TMP_0,

    /// The special `spilltmp1` scratch register.
    SPILL_TMP_1,
}

/// Any register, regardless of class.
///
/// Never appears inside an instruction -- instructions always name a particular
/// class of register -- but this is useful for testing and things like that.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnyReg {
    S(SReg),
    X(XReg),
    F(FReg),
    V(VReg),
}

#[allow(missing_docs)]
impl SReg {
    pub const RANGE: Range<u8> = 0..(Self::SPILL_TMP_1 as u8 + 1);

    pub fn index(self) -> usize {
        usize::from(self as u8)
    }

    pub fn new(index: u8) -> Option<Self> {
        if Self::RANGE.contains(&index) {
            Some(unsafe { Self::new_unchecked(index) })
        } else {
            None
        }
    }

    pub unsafe fn new_unchecked(index: u8) -> Self {
        core::mem::transmute(index)
    }
}

impl fmt::Debug for SReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for SReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SP => write!(f, "sp"),
            Self::LR => write!(f, "lr"),
            Self::FP => write!(f, "fp"),
            Self::SPILL_TMP_0 => write!(f, "spilltmp0"),
            Self::SPILL_TMP_1 => write!(f, "spilltmp1"),
        }
    }
}

impl From<XReg> for AnyReg {
    fn from(x: XReg) -> Self {
        Self::X(x)
    }
}

impl From<SReg> for AnyReg {
    fn from(s: SReg) -> Self {
        Self::S(s)
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
            AnyReg::S(r) => fmt::Display::fmt(r, f),
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

impl fmt::Display for XReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
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

#[cfg(test)]
mod tests {}
