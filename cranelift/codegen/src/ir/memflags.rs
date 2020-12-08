//! Memory operation flags.

use core::fmt;

enum FlagBit {
    // Flags to introduce a limited form of undefined behavior.
    Notrap,
    Aligned,
    Readonly,
    // Flags to specify endianness of the memory access.  If BigEndian is
    // present, the access is big-endian; otherwise, it is little-endian.
    // This flag should always come last in the enumeration.
    BigEndian,
}

// Note: The NAMES array only hold the names of flags for forms of undefined behavior
// Endianness is handled separately.
const NAMES: [&str; 3] = ["notrap", "aligned", "readonly"];
const LITTLEENDIAN: &str = "little";
const BIGENDIAN: &str = "big";

/// Endianness of a memory access.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Endianness {
    /// Little-endian
    Little,
    /// Big-endian
    Big,
}

/// Flags for memory operations like load/store.
///
/// Each of these flags introduce a limited form of undefined behavior. The flags each enable
/// certain optimizations that need to make additional assumptions. Generally, the semantics of a
/// program does not change when a flag is removed, but adding a flag will.
///
/// In addition, the flags determine the endianness of the memory access.  As opposed to the
/// optimization flags defined above, modifying the endianness would always change program
/// semantics, therefore the endianness must always be explicitly specified when constructing
/// a MemFlags value, and cannot be changed later.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct MemFlags {
    bits: u8,
}

impl MemFlags {
    /// Create a new set of flags.  This is initially empty except for the endianness.
    /// Each MemFlags value always determines the endianness of the memory access.
    pub fn new(endianness: Endianness) -> Self {
        Self {
            bits: match endianness {
                Endianness::Little => 0,
                Endianness::Big => 1 << FlagBit::BigEndian as usize,
            },
        }
    }

    /// Create a set of flags representing an access from a "trusted" address, meaning it's
    /// known to be aligned and non-trapping.
    pub fn trusted(endianness: Endianness) -> Self {
        let mut result = Self::new(endianness);
        result.set_notrap();
        result.set_aligned();
        result
    }

    /// Create a set of flags representing a read-only access from a "trusted" address.
    pub fn trusted_readonly(endianness: Endianness) -> Self {
        let mut result = Self::trusted(endianness);
        result.set_readonly();
        result
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
    /// This cannot be used to change the endianness of the memory access.
    pub fn set_by_name(&mut self, name: &str) -> bool {
        match NAMES.iter().position(|&s| s == name) {
            Some(bit) => {
                self.bits |= 1 << bit;
                true
            }
            None => false,
        }
    }

    /// Try to create a new set of flags, where the endianness is determined
    /// from its textual representation.
    pub fn new_from_str(endianness: &str) -> Option<Self> {
        match endianness {
            LITTLEENDIAN => Some(Self::new(Endianness::Little)),
            BIGENDIAN => Some(Self::new(Endianness::Big)),
            _ => None,
        }
    }

    /// Return endianness of the memory access.
    pub fn endianness(self) -> Endianness {
        if self.read(FlagBit::BigEndian) {
            Endianness::Big
        } else {
            Endianness::Little
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

    /// Test if the `readonly` flag is set.
    ///
    /// Loads with this flag have no memory dependencies.
    /// This results in undefined behavior if the dereferenced memory is mutated at any time
    /// between when the function is called and when it is exited.
    pub fn readonly(self) -> bool {
        self.read(FlagBit::Readonly)
    }

    /// Set the `readonly` flag.
    pub fn set_readonly(&mut self) {
        self.set(FlagBit::Readonly)
    }
}

impl fmt::Display for MemFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Always output endianness first ...
        let endianness = match self.endianness() {
            Endianness::Little => LITTLEENDIAN,
            Endianness::Big => BIGENDIAN,
        };
        write!(f, " {}", endianness)?;
        // ... followed by the undefined behavior flags.
        for (i, n) in NAMES.iter().enumerate() {
            if self.bits & (1 << i) != 0 {
                write!(f, " {}", n)?;
            }
        }
        Ok(())
    }
}
