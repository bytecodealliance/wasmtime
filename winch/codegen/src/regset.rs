use crate::isa::reg::Reg;

/// A bit set to track regiter availability.
pub(crate) struct RegSet {
    /// Bitset to track general purpose register availability.
    gpr: u32,
    /// Bitset to track floating-point register availability.
    fpr: u32,
}

impl RegSet {
    /// Create a new register set.
    pub fn new(gpr: u32, fpr: u32) -> Self {
        Self { gpr, fpr }
    }

    /// Request a general purpose register.
    pub fn any_gpr(&mut self) -> Option<Reg> {
        self.gpr_available().then(|| {
            let index = self.gpr.trailing_zeros();
            self.allocate_gpr(index);
            Reg::int(index as usize)
        })
    }

    /// Request a floating point register.
    pub fn any_fpr(&mut self) -> Option<Reg> {
        self.fpr_available().then(|| {
            let index = self.fpr.trailing_zeros();
            self.allocate_fpr(index);
            Reg::float(index as usize)
        })
    }

    /// Request a specific register.
    pub fn reg(&mut self, reg: Reg) -> Option<Reg> {
        let index = reg.hw_enc();
        if reg.is_int() {
            self.named_gpr_available(index as u32).then(|| {
                self.allocate_gpr(index as u32);
                Reg::int(index as usize)
            })
        } else {
            self.named_fpr_available(index as u32).then(|| {
                self.allocate_fpr(index as u32);
                Reg::float(index as usize)
            })
        }
    }

    /// Marks the specified register as available, utilizing the
    /// register class to determine the bitset that requires updating.
    pub fn free(&mut self, reg: Reg) {
        let index = reg.hw_enc() as u32;
        if reg.is_int() {
            self.gpr |= 1 << index;
        } else {
            self.fpr |= 1 << index;
        }
    }

    /// Returns true if the given general purpose register
    /// is available.
    pub fn named_gpr_available(&self, index: u32) -> bool {
        let index = 1 << index;
        (!self.gpr & index) == 0
    }

    /// Returns true if the given floating point register is
    /// available.
    pub fn named_fpr_available(&self, index: u32) -> bool {
        let index = 1 << index;
        (!self.fpr & index) == 0
    }

    fn gpr_available(&self) -> bool {
        self.gpr != 0
    }

    fn fpr_available(&self) -> bool {
        self.fpr != 0
    }

    fn allocate_gpr(&mut self, index: u32) {
        self.gpr &= !(1 << index);
    }

    fn allocate_fpr(&mut self, index: u32) {
        self.fpr &= !(1 << index);
    }
}

#[cfg(test)]
mod tests {
    use super::{Reg, RegSet};

    const UNIVERSE: u32 = (1 << 16) - 1;

    #[test]
    fn test_any_gpr() {
        let mut set = RegSet::new(UNIVERSE, 0);
        for _ in 0..16 {
            let gpr = set.any_gpr();
            assert!(gpr.is_some())
        }

        assert!(!set.gpr_available());
        assert!(set.any_gpr().is_none())
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
        let gpr = set.any_gpr().unwrap();
        set.free(gpr);
        assert!(set.reg(gpr).is_some());
    }
}
