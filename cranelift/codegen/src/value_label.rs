use crate::ir::ValueLabel;
use crate::machinst::Reg;
use crate::HashMap;
use alloc::vec::Vec;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// Value location range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ValueLocRange {
    /// The ValueLoc containing a ValueLabel during this range.
    pub loc: LabelValueLoc,
    /// The start of the range. It is an offset in the generated code.
    pub start: u32,
    /// The end of the range. It is an offset in the generated code.
    pub end: u32,
}

/// The particular location for a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum LabelValueLoc {
    /// New-backend Reg.
    Reg(Reg),
    /// New-backend offset from stack pointer.
    SPOffset(i64),
}

/// Resulting map of Value labels and their ranges/locations.
pub type ValueLabelsRanges = HashMap<ValueLabel, Vec<ValueLocRange>>;
