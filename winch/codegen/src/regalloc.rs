use crate::{isa::reg::Reg, regset::RegSet};

/// The register allocator.
///
/// The register allocator uses a single-pass algorithm;
/// its implementation uses a bitset as a freelist
/// to track per-class register availability.
///
/// If a particular register is not available upon request
/// the register allocation will perform a "spill", essentially
/// moving Local and Register values in the stack to memory.
/// This processs ensures that whenever a register is requested,
/// it is going to be available.
pub(crate) struct RegAlloc {
    pub scratch: Reg,
    regset: RegSet,
}

impl RegAlloc {
    /// Create a new register allocator
    /// from a register set.
    pub fn new(regset: RegSet, scratch: Reg) -> Self {
        Self { regset, scratch }
    }

    /// Allocate the next available general purpose register,
    /// spilling if none available.
    pub fn any_gpr<F>(&mut self, spill: &mut F) -> Reg
    where
        F: FnMut(&mut RegAlloc),
    {
        self.regset.any_gpr().unwrap_or_else(|| {
            spill(self);
            self.regset.any_gpr().expect("any gpr to be available")
        })
    }

    /// Allocate the next available floating point register, spilling
    /// if none available.
    pub fn any_fpr<F>(&mut self, spill: &mut F) -> Reg
    where
        F: FnMut(&mut RegAlloc),
    {
        self.regset.any_fpr().unwrap_or_else(|| {
            spill(self);
            self.regset.any_fpr().expect("any fpr to be available")
        })
    }

    /// Returns true if the specified register is allocatable.
    pub fn reg_available(&self, reg: Reg) -> bool {
        if reg.is_int() {
            self.gpr_available(reg)
        } else {
            self.fpr_available(reg)
        }
    }

    /// Checks if a general purpose register is avaiable.
    pub fn gpr_available(&self, reg: Reg) -> bool {
        self.regset.named_gpr_available(reg.hw_enc() as u32)
    }

    /// Checks if a floating point register is avaiable.
    pub fn fpr_available(&self, reg: Reg) -> bool {
        self.regset.named_fpr_available(reg.hw_enc() as u32)
    }

    /// Request a specific register, spilling if not available.
    pub fn reg<F>(&mut self, named: Reg, mut spill: F) -> Reg
    where
        F: FnMut(&mut RegAlloc),
    {
        // If the scratch register is explicitly requested
        // just return it, it's usage should never cause spills.
        if named == self.scratch {
            return named;
        }

        self.regset.reg(named).unwrap_or_else(|| {
            spill(self);
            self.regset
                .reg(named)
                .expect(&format!("register {:?} to be available", named))
        })
    }

    /// Free the given register.
    pub fn free(&mut self, reg: Reg) {
        if reg != self.scratch {
            self.regset.free(reg);
        }
    }
}
