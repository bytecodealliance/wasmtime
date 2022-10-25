//! AArch64 register definition

use crate::isa::reg::Reg;
use regalloc2::{PReg, RegClass};

/// Construct a X-register from an index.
pub(crate) const fn xreg(num: u8) -> Reg {
    assert!(num < 31);
    Reg::new(PReg::new(num as usize, RegClass::Int))
}

/// Construct a V-register from an index.
pub(crate) const fn vreg(num: u8) -> Reg {
    assert!(num < 32);
    Reg::new(PReg::new(num as usize, RegClass::Float))
}
