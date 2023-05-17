use crate::isa::reg::Reg;

/// A bit set to track regiter availability.
pub(crate) struct RegSet {
    /// Bitset to track general purpose register availability.
    gpr: u32,
    /// Bitset to track floating-point register availability.
    _fpr: u32,
}

impl RegSet {
    /// Create a new register set.
    pub fn new(gpr: u32, fpr: u32) -> Self {
        Self { gpr, _fpr: fpr }
    }

    /// Request a general purpose register.
    pub fn any_gpr(&mut self) -> Option<Reg> {
        self.gpr_available().then(|| {
            let index = self.gpr.trailing_zeros();
            self.allocate(index);
            Reg::int(index as usize)
        })
    }

    /// Request a specific general purpose register.
    pub fn gpr(&mut self, reg: Reg) -> Option<Reg> {
        let index = reg.hw_enc();
        self.named_gpr_available(index as u32).then(|| {
            self.allocate(index as u32);
            Reg::int(index as usize)
        })
    }

    /// Free the given general purpose register.
    pub fn free_gpr(&mut self, reg: Reg) {
        let index = reg.hw_enc() as u32;
        self.gpr |= 1 << index;
    }

    /// Returns true if the given general purpose register
    /// is available.
    pub fn named_gpr_available(&self, index: u32) -> bool {
        let index = 1 << index;
        (!self.gpr & index) == 0
    }

    fn gpr_available(&self) -> bool {
        self.gpr != 0
    }

    fn allocate(&mut self, index: u32) {
        self.gpr &= !(1 << index);
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
        assert!(set.gpr(target).is_none());
    }

    #[test]
    fn test_free_gpr() {
        let mut set = RegSet::new(UNIVERSE, 0);
        let gpr = set.any_gpr().unwrap();
        set.free_gpr(gpr);
        assert!(set.gpr(gpr).is_some());
    }
}
