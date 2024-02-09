//! Memory operation flags.

use core::fmt;

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

enum FlagBit {
    /// Guaranteed not to trap. This may enable additional
    /// optimizations to be performed.
    Notrap,
    /// Guaranteed to use "natural alignment" for the given type. This
    /// may enable better instruction selection.
    Aligned,
    /// A load that reads data in memory that does not change for the
    /// duration of the function's execution. This may enable
    /// additional optimizations to be performed.
    Readonly,
    /// Load multi-byte values from memory in a little-endian format.
    LittleEndian,
    /// Load multi-byte values from memory in a big-endian format.
    BigEndian,
    /// Accesses only the "heap" part of abstract state. Used for
    /// alias analysis. Mutually exclusive with "table" and "vmctx".
    Heap,
    /// Accesses only the "table" part of abstract state. Used for
    /// alias analysis. Mutually exclusive with "heap" and "vmctx".
    Table,
    /// Accesses only the "vmctx" part of abstract state. Used for
    /// alias analysis. Mutually exclusive with "heap" and "table".
    Vmctx,
    /// Check this load or store for safety when using the
    /// proof-carrying-code framework. The address must have a
    /// `PointsTo` fact attached with a sufficiently large valid range
    /// for the accessed size.
    Checked,
}

const NAMES: [&str; 9] = [
    "notrap", "aligned", "readonly", "little", "big", "heap", "table", "vmctx", "checked",
];

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
/// In addition, the flags determine the endianness of the memory access.  By default,
/// any memory access uses the native endianness determined by the target ISA.  This can
/// be overridden for individual accesses by explicitly specifying little- or big-endian
/// semantics via the flags.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemFlags {
    bits: u16,
}

impl MemFlags {
    /// Create a new empty set of flags.
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Create a set of flags representing an access from a "trusted" address, meaning it's
    /// known to be aligned and non-trapping.
    pub const fn trusted() -> Self {
        Self::new().with_notrap().with_aligned()
    }

    /// Read a flag bit.
    const fn read(self, bit: FlagBit) -> bool {
        self.bits & (1 << bit as usize) != 0
    }

    /// Return a new `MemFlags` with this flag bit set.
    const fn with(mut self, bit: FlagBit) -> Self {
        self.bits |= 1 << bit as usize;
        self
    }

    /// Set a flag bit by name.
    ///
    /// Returns true if the flag was found and set, false for an unknown flag name.
    /// Will also return false when trying to set inconsistent endianness flags.
    pub fn set_by_name(&mut self, name: &str) -> bool {
        match NAMES.iter().position(|&s| s == name) {
            Some(bit) => {
                let bits = self.bits | 1 << bit;
                if (bits & (1 << FlagBit::LittleEndian as usize)) != 0
                    && (bits & (1 << FlagBit::BigEndian as usize)) != 0
                {
                    false
                } else {
                    self.bits = bits;
                    true
                }
            }
            None => false,
        }
    }

    /// Return endianness of the memory access.  This will return the endianness
    /// explicitly specified by the flags if any, and will default to the native
    /// endianness otherwise.  The native endianness has to be provided by the
    /// caller since it is not explicitly encoded in CLIF IR -- this allows a
    /// front end to create IR without having to know the target endianness.
    pub const fn endianness(self, native_endianness: Endianness) -> Endianness {
        if self.read(FlagBit::LittleEndian) {
            Endianness::Little
        } else if self.read(FlagBit::BigEndian) {
            Endianness::Big
        } else {
            native_endianness
        }
    }

    /// Set endianness of the memory access.
    pub fn set_endianness(&mut self, endianness: Endianness) {
        *self = self.with_endianness(endianness);
    }

    /// Set endianness of the memory access, returning new flags.
    pub const fn with_endianness(self, endianness: Endianness) -> Self {
        let res = match endianness {
            Endianness::Little => self.with(FlagBit::LittleEndian),
            Endianness::Big => self.with(FlagBit::BigEndian),
        };
        assert!(!(res.read(FlagBit::LittleEndian) && res.read(FlagBit::BigEndian)));
        res
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
    pub const fn notrap(self) -> bool {
        self.read(FlagBit::Notrap)
    }

    /// Set the `notrap` flag.
    pub fn set_notrap(&mut self) {
        *self = self.with_notrap();
    }

    /// Set the `notrap` flag, returning new flags.
    pub const fn with_notrap(self) -> Self {
        self.with(FlagBit::Notrap)
    }

    /// Test if the `aligned` flag is set.
    ///
    /// By default, Cranelift memory instructions work with any unaligned effective address. If the
    /// `aligned` flag is set, the instruction is permitted to trap or return a wrong result if the
    /// effective address is misaligned.
    pub const fn aligned(self) -> bool {
        self.read(FlagBit::Aligned)
    }

    /// Set the `aligned` flag.
    pub fn set_aligned(&mut self) {
        *self = self.with_aligned();
    }

    /// Set the `aligned` flag, returning new flags.
    pub const fn with_aligned(self) -> Self {
        self.with(FlagBit::Aligned)
    }

    /// Test if the `readonly` flag is set.
    ///
    /// Loads with this flag have no memory dependencies.
    /// This results in undefined behavior if the dereferenced memory is mutated at any time
    /// between when the function is called and when it is exited.
    pub const fn readonly(self) -> bool {
        self.read(FlagBit::Readonly)
    }

    /// Set the `readonly` flag.
    pub fn set_readonly(&mut self) {
        *self = self.with_readonly();
    }

    /// Set the `readonly` flag, returning new flags.
    pub const fn with_readonly(self) -> Self {
        self.with(FlagBit::Readonly)
    }

    /// Test if the `heap` bit is set.
    ///
    /// Loads and stores with this flag accesses the "heap" part of
    /// abstract state. This is disjoint from the "table", "vmctx",
    /// and "other" parts of abstract state. In concrete terms, this
    /// means that behavior is undefined if the same memory is also
    /// accessed by another load/store with one of the other
    /// alias-analysis bits (`table`, `vmctx`) set, or `heap` not set.
    pub const fn heap(self) -> bool {
        self.read(FlagBit::Heap)
    }

    /// Set the `heap` bit. See the notes about mutual exclusion with
    /// other bits in `heap()`.
    pub fn set_heap(&mut self) {
        *self = self.with_heap();
    }

    /// Set the `heap` bit, returning new flags.
    pub const fn with_heap(self) -> Self {
        assert!(!self.table() && !self.vmctx());
        self.with(FlagBit::Heap)
    }

    /// Test if the `table` bit is set.
    ///
    /// Loads and stores with this flag accesses the "table" part of
    /// abstract state. This is disjoint from the "heap", "vmctx",
    /// and "other" parts of abstract state. In concrete terms, this
    /// means that behavior is undefined if the same memory is also
    /// accessed by another load/store with one of the other
    /// alias-analysis bits (`heap`, `vmctx`) set, or `table` not set.
    pub const fn table(self) -> bool {
        self.read(FlagBit::Table)
    }

    /// Set the `table` bit. See the notes about mutual exclusion with
    /// other bits in `table()`.
    pub fn set_table(&mut self) {
        *self = self.with_table();
    }

    /// Set the `table` bit, returning new flags.
    pub const fn with_table(self) -> Self {
        assert!(!self.heap() && !self.vmctx());
        self.with(FlagBit::Table)
    }

    /// Test if the `vmctx` bit is set.
    ///
    /// Loads and stores with this flag accesses the "vmctx" part of
    /// abstract state. This is disjoint from the "heap", "table",
    /// and "other" parts of abstract state. In concrete terms, this
    /// means that behavior is undefined if the same memory is also
    /// accessed by another load/store with one of the other
    /// alias-analysis bits (`heap`, `table`) set, or `vmctx` not set.
    pub const fn vmctx(self) -> bool {
        self.read(FlagBit::Vmctx)
    }

    /// Set the `vmctx` bit. See the notes about mutual exclusion with
    /// other bits in `vmctx()`.
    pub fn set_vmctx(&mut self) {
        *self = self.with_vmctx();
    }

    /// Set the `vmctx` bit, returning new flags.
    pub const fn with_vmctx(self) -> Self {
        assert!(!self.heap() && !self.table());
        self.with(FlagBit::Vmctx)
    }

    /// Test if the `checked` bit is set.
    ///
    /// Loads and stores with this flag are verified to access
    /// pointers only with a validated `PointsTo` fact attached, and
    /// with that fact validated, when using the proof-carrying-code
    /// framework. If initial facts on program inputs are correct
    /// (i.e., correctly denote the shape and types of data structures
    /// in memory), and if PCC validates the compiled output, then all
    /// `checked`-marked memory accesses are guaranteed (up to the
    /// checker's correctness) to access valid memory. This can be
    /// used to ensure memory safety and sandboxing.
    pub const fn checked(self) -> bool {
        self.read(FlagBit::Checked)
    }

    /// Set the `checked` bit.
    pub fn set_checked(&mut self) {
        *self = self.with_checked();
    }

    /// Set the `checked` bit, returning new flags.
    pub const fn with_checked(self) -> Self {
        self.with(FlagBit::Checked)
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
