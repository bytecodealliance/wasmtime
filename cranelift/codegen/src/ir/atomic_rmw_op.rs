/// Describes the arithmetic operation in an atomic memory read-modify-write operation.
use core::fmt::{self, Display, Formatter};
use core::str::FromStr;
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
/// Describes the arithmetic operation in an atomic memory read-modify-write operation.
pub enum AtomicRmwOp {
    /// Add
    Add,
    /// Sub
    Sub,
    /// And
    And,
    /// Or
    Or,
    /// Xor
    Xor,
    /// Exchange
    Xchg,
}

impl Display for AtomicRmwOp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match self {
            AtomicRmwOp::Add => "add",
            AtomicRmwOp::Sub => "sub",
            AtomicRmwOp::And => "and",
            AtomicRmwOp::Or => "or",
            AtomicRmwOp::Xor => "xor",
            AtomicRmwOp::Xchg => "xchg",
        };
        f.write_str(s)
    }
}

impl FromStr for AtomicRmwOp {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(AtomicRmwOp::Add),
            "sub" => Ok(AtomicRmwOp::Sub),
            "and" => Ok(AtomicRmwOp::And),
            "or" => Ok(AtomicRmwOp::Or),
            "xor" => Ok(AtomicRmwOp::Xor),
            "xchg" => Ok(AtomicRmwOp::Xchg),
            _ => Err(()),
        }
    }
}
