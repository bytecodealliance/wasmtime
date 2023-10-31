use crate::isa::reg::{Reg, RegClass};

/// A bit set to track regiter availability.
pub(crate) struct RegSet {
    /// Bitset to track general purpose register availability.
    gpr: u32,
    /// Bitset to track floating-point register availability.
    fpr: u32,
}

use std::ops::{Index, IndexMut};

impl Index<RegClass> for RegSet {
    type Output = u32;

    fn index(&self, class: RegClass) -> &Self::Output {
        match class {
            RegClass::Int => &self.gpr,
            RegClass::Float => &self.fpr,
            c => unreachable!("Unexpected register class {:?}", c),
        }
    }
}

impl IndexMut<RegClass> for RegSet {
    fn index_mut(&mut self, class: RegClass) -> &mut Self::Output {
        match class {
            RegClass::Int => &mut self.gpr,
            RegClass::Float => &mut self.fpr,
            c => unreachable!("Unexpected register class {:?}", c),
        }
    }
}

impl RegSet {
    /// Create a new register set.
    pub fn new(gpr: u32, fpr: u32) -> Self {
        Self { gpr, fpr }
    }

    /// Allocate the next available register of the given class,
    /// returning `None` if there are no more registers available.
    pub fn reg_for_class(&mut self, class: RegClass) -> Option<Reg> {
        self.available(class).then(|| {
            let bitset = self[class];
            let index = bitset.trailing_zeros();
            self.allocate(class, index);
            Reg::from(class, index as usize)
        })
    }

    /// Request a specific register.
    pub fn reg(&mut self, reg: Reg) -> Option<Reg> {
        let index = reg.hw_enc();
        self.named_reg_available(reg).then(|| {
            self.allocate(reg.class(), index.into());
            reg
        })
    }

    /// Marks the specified register as available, utilizing the
    /// register class to determine the bitset that requires updating.
    pub fn free(&mut self, reg: Reg) {
        let index = reg.hw_enc() as u32;
        self[reg.class()] |= 1 << index;
    }

    /// Returns true if the specified register is allocatable.
    pub fn named_reg_available(&self, reg: Reg) -> bool {
        let bitset = self[reg.class()];
        let index = 1 << reg.hw_enc();
        (!bitset & index) == 0
    }

    fn available(&self, class: RegClass) -> bool {
        let bitset = self[class];
        bitset != 0
    }

    fn allocate(&mut self, class: RegClass, index: u32) {
        self[class] &= !(1 << index);
    }
}

#[cfg(test)]
mod tests {
    use super::{Reg, RegClass, RegSet};

    const UNIVERSE: u32 = (1 << 16) - 1;

    #[test]
    fn test_any_gpr() {
        let mut set = RegSet::new(UNIVERSE, 0);
        for _ in 0..16 {
            let gpr = set.reg_for_class(RegClass::Int);
            assert!(gpr.is_some())
        }

        assert!(!set.available(RegClass::Int));
        assert!(set.reg_for_class(RegClass::Int).is_none())
    }

    #[test]
    fn test_gpr() {
        let all = UNIVERSE & !(1 << 5);
        let target = Reg::int(5);
        let mut set = RegSet::new(all, 0);
        assert!(set.reg(target).is_none());
    }

    #[test]
    fn test_free_reg() {
        let mut set = RegSet::new(UNIVERSE, 0);
        let gpr = set.reg_for_class(RegClass::Int).unwrap();
        set.free(gpr);
        assert!(set.reg(gpr).is_some());
    }
}
