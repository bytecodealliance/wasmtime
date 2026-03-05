/// Describes the arithmetic operation in an atomic memory read-modify-write operation.
use core::fmt::{self, Display, Formatter};
use core::str::FromStr;
#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

use crate::ir::AtomicOrdering;

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
    /// Nand
    Nand,
    /// Or
    Or,
    /// Xor
    Xor,
    /// Exchange
    Xchg,
    /// Unsigned min
    Umin,
    /// Unsigned max
    Umax,
    /// Signed min
    Smin,
    /// Signed max
    Smax,
}

impl AtomicRmwOp {
    /// Returns a slice with all supported [AtomicRmwOp]'s.
    pub fn all() -> &'static [AtomicRmwOp] {
        &[
            AtomicRmwOp::Add,
            AtomicRmwOp::Sub,
            AtomicRmwOp::And,
            AtomicRmwOp::Nand,
            AtomicRmwOp::Or,
            AtomicRmwOp::Xor,
            AtomicRmwOp::Xchg,
            AtomicRmwOp::Umin,
            AtomicRmwOp::Umax,
            AtomicRmwOp::Smin,
            AtomicRmwOp::Smax,
        ]
    }

    pub(crate) fn to_u8(&self) -> u8 {
        match &self {
            AtomicRmwOp::Add => 0,
            AtomicRmwOp::Sub => 1,
            AtomicRmwOp::And => 2,
            AtomicRmwOp::Nand => 3,
            AtomicRmwOp::Or => 4,
            AtomicRmwOp::Xor => 5,
            AtomicRmwOp::Xchg => 6,
            AtomicRmwOp::Umin => 7,
            AtomicRmwOp::Umax => 8,
            AtomicRmwOp::Smin => 9,
            AtomicRmwOp::Smax => 10,
        }
    }

    pub(crate) fn from_u8(data: u8) -> AtomicRmwOp {
        match data {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::And,
            3 => Self::Nand,
            4 => Self::Or,
            5 => Self::Xor,
            6 => Self::Xchg,
            7 => Self::Umin,
            8 => Self::Umax,
            9 => Self::Smin,
            10 => Self::Smax,
            _ => unreachable!(),
        }
    }
}

impl Display for AtomicRmwOp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match self {
            AtomicRmwOp::Add => "add",
            AtomicRmwOp::Sub => "sub",
            AtomicRmwOp::And => "and",
            AtomicRmwOp::Nand => "nand",
            AtomicRmwOp::Or => "or",
            AtomicRmwOp::Xor => "xor",
            AtomicRmwOp::Xchg => "xchg",
            AtomicRmwOp::Umin => "umin",
            AtomicRmwOp::Umax => "umax",
            AtomicRmwOp::Smin => "smin",
            AtomicRmwOp::Smax => "smax",
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
            "nand" => Ok(AtomicRmwOp::Nand),
            "or" => Ok(AtomicRmwOp::Or),
            "xor" => Ok(AtomicRmwOp::Xor),
            "xchg" => Ok(AtomicRmwOp::Xchg),
            "umin" => Ok(AtomicRmwOp::Umin),
            "umax" => Ok(AtomicRmwOp::Umax),
            "smin" => Ok(AtomicRmwOp::Smin),
            "smax" => Ok(AtomicRmwOp::Smax),
            _ => Err(()),
        }
    }
}

/// Atomic Read-Modify-Write Options
/// Describes the ordering as well as the option
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct AtomicRmwData(
    // bits 0/1/2/3/4 = AtomicRmwOp
    // 5/6/7 = AtomicOrdering
    pub(crate) u8,
);

impl AtomicRmwData {
    const MASK_RMW: u8 = 0b0001_1111;
    const MASK_ORDERING: u8 = 0b1110_0000;

    /// Creates an AtomicRmwData structure with the provided [AtomicOrdering] and [AtomicRmwData]
    pub fn new(ordering: AtomicOrdering, op: AtomicRmwOp) -> Self {
        let rmw = op.to_u8();

        Self(rmw | (ordering.to_u8() << 5))
    }

    /// Gets the associated [AtomicOrdering]
    pub fn ordering(&self) -> AtomicOrdering {
        AtomicOrdering::from_u8(self.0 >> 5)
    }

    /// Sets the associated [AtomicOrdering]
    pub fn set_ordering(&mut self, ordering: AtomicOrdering) {
        self.0 &= Self::MASK_RMW;
        self.0 |= ordering.to_u8() << 5;
    }

    /// Gets the associated [AtomicRmwOp]
    pub fn op(&self) -> AtomicRmwOp {
        AtomicRmwOp::from_u8(self.0 & Self::MASK_RMW)
    }

    /// Sets the associated [AtomicRmwOp]
    pub fn set_op(&mut self, op: AtomicRmwOp) {
        self.0 &= Self::MASK_ORDERING;
        self.0 |= op.to_u8();
    }
}

impl Display for AtomicRmwData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.op().fmt(f)?;
        f.write_str(" ")?;
        self.ordering().fmt(f)
    }
}

#[cfg(feature = "enable-serde")]
impl serde::Serialize for AtomicRmwData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AtomicRmwData", 2)?;
        s.serialize_field("op", &self.op())?;
        s.serialize_field("ordering", &self.ordering())?;
        s.end()
    }
}

#[cfg(feature = "enable-serde")]
impl<'de> serde::Deserialize<'de> for AtomicRmwData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Readable {
            op: AtomicRmwOp,
            ordering: AtomicOrdering,
        }

        let r = Readable::deserialize(deserializer)?;
        Ok(Self::new(r.ordering, r.op))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_parse() {
        for op in AtomicRmwOp::all() {
            let roundtripped = format!("{op}").parse::<AtomicRmwOp>().unwrap();
            assert_eq!(*op, roundtripped);
        }
    }

    #[test]
    fn check_bitpacking() {
        for op in AtomicRmwOp::all() {
            for ordering in AtomicOrdering::all() {
                let mut data = AtomicRmwData::new(*ordering, *op);

                assert_eq!(data.op(), *op);
                assert_eq!(data.ordering(), *ordering);

                // Test if the individual masks are correct
                data.0 = 0;

                data.set_op(*op);
                data.set_ordering(*ordering);

                assert_eq!(data.op(), *op);
                assert_eq!(data.ordering(), *ordering);
            }
        }
    }
}
