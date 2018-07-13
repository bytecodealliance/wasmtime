//! Memory operation flags.

use std::fmt;

enum FlagBit {
    Notrap,
    Aligned,
}

const NAMES: [&str; 2] = ["notrap", "aligned"];

/// Flags for memory operations like load/store.
///
/// Each of these flags introduce a limited form of undefined behavior. The flags each enable
/// certain optimizations that need to make additional assumptions. Generally, the semantics of a
/// program does not change when a flag is removed, but adding a flag will.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct MemFlags {
    bits: u8,
}

impl MemFlags {
    /// Create a new empty set of flags.
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    /// Read a flag bit.
    fn read(self, bit: FlagBit) -> bool {
        self.bits & (1 << bit as usize) != 0
    }

    /// Set a flag bit.
    fn set(&mut self, bit: FlagBit) {
        self.bits |= 1 << bit as usize
    }

    /// Set a flag bit by name.
    ///
    /// Returns true if the flag was found and set, false for an unknown flag name.
    pub fn set_by_name(&mut self, name: &str) -> bool {
        match NAMES.iter().position(|&s| s == name) {
            Some(bit) => {
                self.bits |= 1 << bit;
                true
            }
            None => false,
        }
    }

    /// Test if the `notrap` flag is set.
    ///
    /// Normally, trapping is part of the semantics of a load/store operation. If the platform
    /// would cause a trap when accessing the effective address, the Cranelift memory operation is
    /// also required to trap.
    ///
    /// The `notrap` flag tells Cranelift that the memory is *accessible*, which means that
    /// accesses will not trap. This makes it possible to delete an unused load or a dead store
    /// instruction.
    pub fn notrap(self) -> bool {
        self.read(FlagBit::Notrap)
    }

    /// Set the `notrap` flag.
    pub fn set_notrap(&mut self) {
        self.set(FlagBit::Notrap)
    }

    /// Test if the `aligned` flag is set.
    ///
    /// By default, Cranelift memory instructions work with any unaligned effective address. If the
    /// `aligned` flag is set, the instruction is permitted to trap or return a wrong result if the
    /// effective address is misaligned.
    pub fn aligned(self) -> bool {
        self.read(FlagBit::Aligned)
    }

    /// Set the `aligned` flag.
    pub fn set_aligned(&mut self) {
        self.set(FlagBit::Aligned)
    }
}

impl fmt::Display for MemFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, n) in NAMES.iter().enumerate() {
            if self.bits & (1 << i) != 0 {
                write!(f, " {}", n)?;
            }
        }
        Ok(())
    }
}
