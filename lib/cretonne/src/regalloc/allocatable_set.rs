//! Set of allocatable registers as a bit vector of register units.
//!
//! While allocating registers, we need to keep track of which registers are available and which
//! registers are in use. Since registers can alias in different ways, we track this via the
//! "register unit" abstraction. Every register contains one or more register units. Registers that
//! share a register unit can't be in use at the same time.

use std::mem::size_of_val;
use isa::registers::{RegUnit, RegUnitMask, RegClass};

/// Set of registers available for allocation.
#[derive(Clone)]
pub struct AllocatableSet {
    avail: RegUnitMask,
}

// Given a register class and a register unit in the class, compute a word index and a bit mask of
// register units representing that register.
//
// Note that a register is not allowed to straddle words.
fn bitmask(rc: RegClass, reg: RegUnit) -> (usize, u32) {
    // Bit mask representing the register. It is `rc.width` consecutive units.
    let width_bits = (1 << rc.width) - 1;
    // Index into avail[] of the word containing `reg`.
    let word_index = (reg / 32) as usize;
    // The actual bits in the word that cover `reg`.
    let reg_bits = width_bits << (reg % 32);

    (word_index, reg_bits)
}

impl AllocatableSet {
    /// Create a new register set with all registers available.
    ///
    /// Note that this includes *all* registers. Query the `TargetIsa` object to get a set of
    /// allocatable registers where reserved registers have been filtered out.
    pub fn new() -> AllocatableSet {
        AllocatableSet { avail: [!0; 3] }
    }

    /// Returns `true` if the specified register is available.
    pub fn is_avail(&self, rc: RegClass, reg: RegUnit) -> bool {
        let (idx, bits) = bitmask(rc, reg);
        (self.avail[idx] & bits) == bits
    }

    /// Allocate `reg` from `rc` so it is no longer available.
    ///
    /// It is an error to take a register that doesn't have all of its register units available.
    pub fn take(&mut self, rc: RegClass, reg: RegUnit) {
        let (idx, bits) = bitmask(rc, reg);
        debug_assert!((self.avail[idx] & bits) == bits, "Not available");
        self.avail[idx] &= !bits;
    }

    /// Make `reg` available for allocation again.
    pub fn free(&mut self, rc: RegClass, reg: RegUnit) {
        let (idx, bits) = bitmask(rc, reg);
        debug_assert!((self.avail[idx] & bits) == 0, "Not allocated");
        self.avail[idx] |= bits;
    }

    /// Return an iterator over all available registers belonging to the register class `rc`.
    ///
    /// This doesn't allocate anything from the set; use `take()` for that.
    pub fn iter(&self, rc: RegClass) -> RegSetIter {
        // Start by copying the RC mask. It is a single set bit for each register in the class.
        let mut rsi = RegSetIter { regs: rc.mask };

        // Mask out the unavailable units.
        for idx in 0..self.avail.len() {
            // If a single unit in a register is unavailable, the whole register can't be used.
            // If a register straddles a word boundary, it will be marked as unavailable.
            // There's an assertion in `cdsl/registers.py` to check for that.
            for i in 0..rc.width {
                rsi.regs[idx] &= self.avail[idx] >> i;
            }
        }
        rsi
    }
}

/// Iterator over available registers in a register class.
pub struct RegSetIter {
    regs: RegUnitMask,
}

impl Iterator for RegSetIter {
    type Item = RegUnit;

    fn next(&mut self) -> Option<RegUnit> {
        let mut unit_offset = 0;

        // Find the first set bit in `self.regs`.
        for word in &mut self.regs {
            if *word != 0 {
                // Compute the register unit number from the lowest set bit in the word.
                let unit = unit_offset + word.trailing_zeros() as RegUnit;

                // Clear that lowest bit so we won't find it again.
                *word = *word & (*word - 1);

                return Some(unit);
            }
            // How many register units was there in the word? This is a constant 32 for `u32` etc.
            unit_offset += 8 * size_of_val(word) as RegUnit;
        }

        // All of `self.regs` is 0.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use isa::registers::{RegClass, RegClassData};

    // Register classes for testing.
    const GPR: RegClass = &RegClassData {
        index: 0,
        width: 1,
        subclasses: 0,
        mask: [0xf0000000, 0x0000000f, 0],
    };
    const DPR: RegClass = &RegClassData {
        index: 0,
        width: 2,
        subclasses: 0,
        mask: [0x50000000, 0x0000000a, 0],
    };

    #[test]
    fn put_and_take() {
        let mut regs = AllocatableSet::new();

        // `GPR` has units 28-36.
        assert_eq!(regs.iter(GPR).count(), 8);
        assert_eq!(regs.iter(DPR).collect::<Vec<_>>(), [28, 30, 33, 35]);

        assert!(regs.is_avail(GPR, 29));
        regs.take(&GPR, 29);
        assert!(!regs.is_avail(GPR, 29));

        assert_eq!(regs.iter(GPR).count(), 7);
        assert_eq!(regs.iter(DPR).collect::<Vec<_>>(), [30, 33, 35]);

        assert!(regs.is_avail(GPR, 30));
        regs.take(&GPR, 30);
        assert!(!regs.is_avail(GPR, 30));

        assert_eq!(regs.iter(GPR).count(), 6);
        assert_eq!(regs.iter(DPR).collect::<Vec<_>>(), [33, 35]);

        assert!(regs.is_avail(GPR, 32));
        regs.take(&GPR, 32);
        assert!(!regs.is_avail(GPR, 32));

        assert_eq!(regs.iter(GPR).count(), 5);
        assert_eq!(regs.iter(DPR).collect::<Vec<_>>(), [33, 35]);

        regs.free(&GPR, 30);
        assert!(regs.is_avail(GPR, 30));
        assert!(!regs.is_avail(GPR, 29));
        assert!(!regs.is_avail(GPR, 32));

        assert_eq!(regs.iter(GPR).count(), 6);
        assert_eq!(regs.iter(DPR).collect::<Vec<_>>(), [30, 33, 35]);

        regs.free(&GPR, 32);
        assert!(regs.is_avail(GPR, 31));
        assert!(!regs.is_avail(GPR, 29));
        assert!(regs.is_avail(GPR, 32));

        assert_eq!(regs.iter(GPR).count(), 7);
        assert_eq!(regs.iter(DPR).collect::<Vec<_>>(), [30, 33, 35]);
    }
}
