/// Aarch register definition
use regalloc2::{PReg, RegClass};

/// Construct a X-register from an index
pub(crate) const fn xreg(num: u8) -> PReg {
    assert!(num < 31);
    PReg::new(num as usize, RegClass::Int)
}

/// Construct a V-register from an index
pub(crate) const fn vreg(num: u8) -> PReg {
    assert!(num < 32);
    PReg::new(num as usize, RegClass::Float)
}
