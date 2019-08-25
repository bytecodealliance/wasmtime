//! Set of allocatable registers as a bit vector of register units.
//!
//! While allocating registers, we need to keep track of which registers are available and which
//! registers are in use. Since registers can alias in different ways, we track this via the
//! "register unit" abstraction. Every register contains one or more register units. Registers that
//! share a register unit can't be in use at the same time.

use crate::isa::registers::{RegClass, RegInfo, RegUnit, RegUnitMask};
use core::char;
use core::fmt;
use core::iter::ExactSizeIterator;
use core::mem::size_of_val;

/// Set of registers available for allocation.
#[derive(Clone)]
pub struct RegisterSet {
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

impl RegisterSet {
    /// Create a new register set with all registers available.
    ///
    /// Note that this includes *all* registers. Query the `TargetIsa` object to get a set of
    /// allocatable registers where reserved registers have been filtered out.
    pub fn new() -> Self {
        Self { avail: [!0; 3] }
    }

    /// Create a new register set with no registers available.
    pub fn empty() -> Self {
        Self { avail: [0; 3] }
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
        debug_assert!(
            (self.avail[idx] & bits) == bits,
            "{}:{} not available in {}",
            rc,
            rc.info.display_regunit(reg),
            self.display(rc.info)
        );
        self.avail[idx] &= !bits;
    }

    /// Return `reg` and all of its register units to the set of available registers.
    pub fn free(&mut self, rc: RegClass, reg: RegUnit) {
        let (idx, bits) = bitmask(rc, reg);
        debug_assert!(
            (self.avail[idx] & bits) == 0,
            "{}:{} is already free in {}",
            rc,
            rc.info.display_regunit(reg),
            self.display(rc.info)
        );
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
            // If a single unit in a register is unavailable, the whole register can't be used.  If
            // a register straddles a word boundary, it will be marked as unavailable.  There's an
            // assertion in `cranelift-codegen/meta/src/cdsl/regs.rs` to check for that.
            for i in 0..rc.width {
                rsi.regs[idx] &= self.avail[idx] >> i;
            }
        }
        rsi
    }

    /// Check if any register units allocated out of this set interferes with units allocated out
    /// of `other`.
    ///
    /// This assumes that unused bits are 1.
    pub fn interferes_with(&self, other: &Self) -> bool {
        self.avail
            .iter()
            .zip(&other.avail)
            .any(|(&x, &y)| (x | y) != !0)
    }

    /// Intersect this set of registers with `other`. This has the effect of removing any register
    /// units from this set that are not in `other`.
    pub fn intersect(&mut self, other: &Self) {
        for (x, &y) in self.avail.iter_mut().zip(&other.avail) {
            *x &= y;
        }
    }

    /// Return an object that can display this register set, using the register info from the
    /// target ISA.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(&self, regs: R) -> DisplayRegisterSet<'a> {
        DisplayRegisterSet(self.clone(), regs.into())
    }
}

/// Iterator over available registers in a register class.
#[derive(Clone)]
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
                *word &= *word - 1;

                return Some(unit);
            }
            // How many register units was there in the word? This is a constant 32 for `u32` etc.
            unit_offset += 8 * size_of_val(word) as RegUnit;
        }

        // All of `self.regs` is 0.
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let bits = self.regs.iter().map(|&w| w.count_ones() as usize).sum();
        (bits, Some(bits))
    }
}

impl RegSetIter {
    pub fn rnext(&mut self) -> Option<RegUnit> {
        let num_words = self.regs.len();
        let bits_per_word = 8 * size_of_val(&self.regs[0]);

        // Find the last set bit in `self.regs`.
        for i in 0..num_words {
            let word_ix = num_words - 1 - i;

            let word = &mut self.regs[word_ix];
            if *word != 0 {
                let lzeroes = word.leading_zeros() as usize;

                // Clear that highest bit so we won't find it again.
                *word &= !(1 << (bits_per_word - 1 - lzeroes));

                return Some((word_ix * bits_per_word + bits_per_word - 1 - lzeroes) as RegUnit);
            }
        }

        // All of `self.regs` is 0.
        None
    }
}

impl ExactSizeIterator for RegSetIter {}

/// Displaying an `RegisterSet` correctly requires the associated `RegInfo` from the target ISA.
pub struct DisplayRegisterSet<'a>(RegisterSet, Option<&'a RegInfo>);

impl<'a> fmt::Display for DisplayRegisterSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        match self.1 {
            None => {
                for w in &self.0.avail {
                    write!(f, " #{:08x}", w)?;
                }
            }
            Some(reginfo) => {
                let toprcs = reginfo
                    .banks
                    .iter()
                    .map(|b| b.first_toprc + b.num_toprcs)
                    .max()
                    .expect("No register banks");
                for rc in &reginfo.classes[0..toprcs] {
                    if rc.width == 1 {
                        let bank = &reginfo.banks[rc.bank as usize];
                        write!(f, " {}: ", rc)?;
                        for offset in 0..bank.units {
                            let reg = bank.first_unit + offset;
                            if !rc.contains(reg) {
                                continue;
                            }
                            if !self.0.is_avail(rc, reg) {
                                write!(f, "-")?;
                                continue;
                            }
                            // Display individual registers as either the second letter of their
                            // name or the last digit of their number.
                            // This works for x86 (rax, rbx, ...) and for numbered regs.
                            write!(
                                f,
                                "{}",
                                bank.names
                                    .get(offset as usize)
                                    .and_then(|name| name.chars().nth(1))
                                    .unwrap_or_else(|| char::from_digit(
                                        u32::from(offset % 10),
                                        10
                                    )
                                    .unwrap())
                            )?;
                        }
                    }
                }
            }
        }
        write!(f, " ]")
    }
}

impl fmt::Display for RegisterSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(None).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::registers::{RegClass, RegClassData};
    use std::vec::Vec;

    // Register classes for testing.
    const GPR: RegClass = &RegClassData {
        name: "GPR",
        index: 0,
        width: 1,
        bank: 0,
        toprc: 0,
        first: 28,
        subclasses: 0,
        mask: [0xf0000000, 0x0000000f, 0],
        info: &INFO,
    };

    const DPR: RegClass = &RegClassData {
        name: "DPR",
        index: 0,
        width: 2,
        bank: 0,
        toprc: 0,
        first: 28,
        subclasses: 0,
        mask: [0x50000000, 0x0000000a, 0],
        info: &INFO,
    };

    const INFO: RegInfo = RegInfo {
        banks: &[],
        classes: &[],
    };

    const RSI_1: RegSetIter = RegSetIter {
        regs: [0x31415927, 0x27182818, 0x14141356],
    };

    const RSI_2: RegSetIter = RegSetIter {
        regs: [0x00000000, 0x00000000, 0x00000000],
    };

    const RSI_3: RegSetIter = RegSetIter {
        regs: [0xffffffff, 0xffffffff, 0xffffffff],
    };

    fn reverse_regset_iteration_work(rsi: &RegSetIter) {
        // Check the reverse iterator by comparing its output with the forward iterator.
        let rsi_f = (*rsi).clone();
        let results_f = rsi_f.collect::<Vec<_>>();

        let mut rsi_r = (*rsi).clone();
        let mut results_r = Vec::<RegUnit>::new();
        while let Some(r) = rsi_r.rnext() {
            results_r.push(r);
        }

        let len_f = results_f.len();
        let len_r = results_r.len();
        assert_eq!(len_f, len_r);

        for i in 0..len_f {
            assert_eq!(results_f[i], results_r[len_f - 1 - i]);
        }
    }

    #[test]
    fn reverse_regset_iteration() {
        reverse_regset_iteration_work(&RSI_1);
        reverse_regset_iteration_work(&RSI_2);
        reverse_regset_iteration_work(&RSI_3);
    }

    #[test]
    fn put_and_take() {
        let mut regs = RegisterSet::new();

        // `GPR` has units 28-36.
        assert_eq!(regs.iter(GPR).len(), 8);
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

    #[test]
    fn interference() {
        let mut regs1 = RegisterSet::new();
        let mut regs2 = RegisterSet::new();

        assert!(!regs1.interferes_with(&regs2));
        regs1.take(&GPR, 32);
        assert!(!regs1.interferes_with(&regs2));
        regs2.take(&GPR, 31);
        assert!(!regs1.interferes_with(&regs2));
        regs1.intersect(&regs2);
        assert!(regs1.interferes_with(&regs2));
    }
}
