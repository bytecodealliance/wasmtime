//! Program points.

use crate::ir::{Block, Inst, ValueDef};
use core::fmt;

/// An expanded program point directly exposes the variants, but takes twice the space to
/// represent.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ExpandedProgramPoint {
    /// An instruction in the function.
    Inst(Inst),
    /// A block header.
    Block(Block),
}

impl ExpandedProgramPoint {
    /// Get the instruction we know is inside.
    pub fn unwrap_inst(self) -> Inst {
        match self {
            Self::Inst(x) => x,
            Self::Block(x) => panic!("expected inst: {}", x),
        }
    }
}

impl From<Inst> for ExpandedProgramPoint {
    fn from(inst: Inst) -> Self {
        Self::Inst(inst)
    }
}

impl From<Block> for ExpandedProgramPoint {
    fn from(block: Block) -> Self {
        Self::Block(block)
    }
}

impl From<ValueDef> for ExpandedProgramPoint {
    fn from(def: ValueDef) -> Self {
        match def {
            ValueDef::Result(inst, _) => inst.into(),
            ValueDef::Param(block, _) => block.into(),
            ValueDef::Union(_, _) => panic!("Union does not have a single program point"),
        }
    }
}

impl fmt::Display for ExpandedProgramPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Inst(x) => write!(f, "{}", x),
            Self::Block(x) => write!(f, "{}", x),
        }
    }
}

impl fmt::Debug for ExpandedProgramPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExpandedProgramPoint({})", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityRef;
    use crate::ir::{Block, Inst};
    use alloc::string::ToString;

    #[test]
    fn convert() {
        let i5 = Inst::new(5);
        let b3 = Block::new(3);

        let pp1: ExpandedProgramPoint = i5.into();
        let pp2: ExpandedProgramPoint = b3.into();

        assert_eq!(pp1.to_string(), "inst5");
        assert_eq!(pp2.to_string(), "block3");
    }
}
