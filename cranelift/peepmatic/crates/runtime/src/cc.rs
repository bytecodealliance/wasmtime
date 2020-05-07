//! Condition codes.

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

/// A condition code.
///
/// This is a special kind of immediate for `icmp` instructions that dictate
/// which parts of the comparison result we care about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum ConditionCode {
    /// Equal.
    // NB: We convert `ConditionCode` into `NonZeroU32`s with unchecked
    // conversions; memory safety relies on no variant being zero.
    Eq = 1,

    /// Not equal.
    Ne,

    /// Signed less than.
    Slt,

    /// Unsigned less than.
    Ult,

    /// Signed greater than or equal.
    Sge,

    /// Unsigned greater than or equal.
    Uge,

    /// Signed greater than.
    Sgt,

    /// Unsigned greater than.
    Ugt,

    /// Signed less than or equal.
    Sle,

    /// Unsigned less than or equal.
    Ule,

    /// Overflow.
    Of,

    /// No overflow.
    Nof,
}

impl TryFrom<u32> for ConditionCode {
    type Error = &'static str;

    fn try_from(x: u32) -> Result<Self, Self::Error> {
        Ok(match x {
            x if Self::Eq as u32 == x => Self::Eq,
            x if Self::Ne as u32 == x => Self::Ne,
            x if Self::Slt as u32 == x => Self::Slt,
            x if Self::Ult as u32 == x => Self::Ult,
            x if Self::Sge as u32 == x => Self::Sge,
            x if Self::Uge as u32 == x => Self::Uge,
            x if Self::Sgt as u32 == x => Self::Sgt,
            x if Self::Ugt as u32 == x => Self::Ugt,
            x if Self::Sle as u32 == x => Self::Sle,
            x if Self::Ule as u32 == x => Self::Ule,
            x if Self::Of as u32 == x => Self::Of,
            x if Self::Nof as u32 == x => Self::Nof,
            _ => return Err("not a valid condition code value"),
        })
    }
}

impl fmt::Display for ConditionCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eq => write!(f, "eq"),
            Self::Ne => write!(f, "ne"),
            Self::Slt => write!(f, "slt"),
            Self::Ult => write!(f, "ult"),
            Self::Sge => write!(f, "sge"),
            Self::Uge => write!(f, "uge"),
            Self::Sgt => write!(f, "sgt"),
            Self::Ugt => write!(f, "ugt"),
            Self::Sle => write!(f, "sle"),
            Self::Ule => write!(f, "ule"),
            Self::Of => write!(f, "of"),
            Self::Nof => write!(f, "nof"),
        }
    }
}
