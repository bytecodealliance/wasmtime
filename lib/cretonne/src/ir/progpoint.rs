//! Program points.

use entity_map::EntityRef;
use ir::{Ebb, Inst};
use std::fmt;
use std::u32;
use std::cmp;

/// A `ProgramPoint` represents a position in a function where the live range of an SSA value can
/// begin or end. It can be either:
///
/// 1. An instruction or
/// 2. An EBB header.
///
/// This corresponds more or less to the lines in the textual representation of Cretonne IL.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ProgramPoint(u32);

impl From<Inst> for ProgramPoint {
    fn from(inst: Inst) -> ProgramPoint {
        let idx = inst.index();
        assert!(idx < (u32::MAX / 2) as usize);
        ProgramPoint((idx * 2) as u32)
    }
}

impl From<Ebb> for ProgramPoint {
    fn from(ebb: Ebb) -> ProgramPoint {
        let idx = ebb.index();
        assert!(idx < (u32::MAX / 2) as usize);
        ProgramPoint((idx * 2 + 1) as u32)
    }
}

/// An expanded program point directly exposes the variants, but takes twice the space to
/// represent.
pub enum ExpandedProgramPoint {
    // An instruction in the function.
    Inst(Inst),
    // An EBB header.
    Ebb(Ebb),
}

impl ProgramPoint {
    /// Expand compact program point representation.
    pub fn expand(self) -> ExpandedProgramPoint {
        if self.0 & 1 == 0 {
            ExpandedProgramPoint::Inst(Inst::new((self.0 / 2) as usize))
        } else {
            ExpandedProgramPoint::Ebb(Ebb::new((self.0 / 2) as usize))
        }
    }
}

impl fmt::Display for ProgramPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.expand() {
            ExpandedProgramPoint::Inst(x) => write!(f, "{}", x),
            ExpandedProgramPoint::Ebb(x) => write!(f, "{}", x),
        }
    }
}

impl fmt::Debug for ProgramPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ProgramPoint({})", self)
    }
}

/// Context for ordering program points.
///
/// `ProgramPoint` objects don't carry enough information to be ordered independently, they need a
/// context providing the program order.
pub trait ProgramOrder {
    /// Compare the program points `a` and `b` relative to this program order. Return `Less` if `a`
    /// appears in the program before `b`.
    fn cmp(&self, a: ProgramPoint, b: ProgramPoint) -> cmp::Ordering;
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity_map::EntityRef;
    use ir::{Inst, Ebb};

    #[test]
    fn convert() {
        let i5 = Inst::new(5);
        let b3 = Ebb::new(3);

        let pp1: ProgramPoint = i5.into();
        let pp2: ProgramPoint = b3.into();

        assert_eq!(pp1.to_string(), "inst5");
        assert_eq!(pp2.to_string(), "ebb3");
    }
}
