//! x64 addressing mode.

use crate::reg::Reg;

/// Memory address representation.
#[derive(Debug, Copy, Clone)]
pub(crate) enum Address {
    /// Base register with an immediate offset.
    Offset { base: Reg, offset: u32 },
}

impl Address {
    /// Create an offset.
    pub fn offset(base: Reg, offset: u32) -> Self {
        Self::Offset { base, offset }
    }
}
