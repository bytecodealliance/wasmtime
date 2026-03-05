use crate::ir::{AliasRegion, AtomicOrdering, Endianness, MemFlags, TrapCode};
use core::num::NonZeroU8;
use core::str::FromStr;

/// Flags for AtomicCas instruction
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
#[cfg_attr(feature = "enable-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtomicCasMemFlags {
    // Initialized to all zeros to have all flags have their default value.
    // This is interpreted through various methods below. Currently the bits of
    // this are defined as:
    //
    // * 0/1/2/3/4/5/6/7 - trap code
    // * 8/9/10/11/12/13/14/15 - Permutation based storage
    //
    // Permutation based storage
    //
    // * atomic ordering (5 states)
    // * alias region (3 states)
    // * endianness (3 states)
    // * aligned flag (2 states)
    // * checked bit (2 states)
    //
    // So, mathematically, its `180` different states
    //
    // Current properties upheld are:
    //
    // * only one of little/big endian is set
    // * only one alias region can be set - once set it cannot be changed
    bits: u16,
}

// The "Weights" for our Mixed-Radix system
const WEIGHT_BEFORE_ORDERING: u16 = 180; // 5 * 3 * 3 * 2 * 2

const WEIGHT_ORDERING: u16 = 36; // 3 * 3 * 2 * 2
const WEIGHT_ALIAS_REGION: u16 = 12; // 3 * 2 * 2
const WEIGHT_ENDIANNESS: u16 = 4; // 2 * 2

const WEIGHT_ALIGNED: u16 = 2; // 2
const WEIGHT_CHECKED: u16 = 1;

/// Trap code, if any, for this memory operation.
const MASK_TRAP_CODE: u16 = 0b1111_1111 << TRAP_CODE_OFFSET;
const TRAP_CODE_OFFSET: u16 = 0;

impl AtomicCasMemFlags {
    /// Create a new empty set of flags.
    pub const fn new(ordering: AtomicOrdering) -> Self {
        Self { bits: 0 }
            .with_ordering(ordering)
            .with_trap_code(Some(TrapCode::HEAP_OUT_OF_BOUNDS))
    }

    /// Create a set of flags representing an access from a "trusted" address, meaning it's
    /// known to be aligned and non-trapping.
    pub const fn trusted(ordering: AtomicOrdering) -> Self {
        Self::new(ordering).with_notrap().with_aligned()
    }

    /// Read a state as encoded
    const fn read_state(self, state_const: u16, before_state_const: u16) -> u8 {
        let higher_state = self.bits >> 8;

        ((higher_state % before_state_const) / state_const)
    }

    /// Return a new `AtomicCasMemFlags` with this flag bit set.
    const fn with_state(mut self, data: u16, state_const: u16, before_state_const: u16) -> Self {
        let out = (self.bits >> 8) - self.read_state(state_const, before_state_const) + data*state_const;

        self.bits &= 0x00FF; // Mask the lower bits
        self.bits |= out << 8;
        self
    }

    /// Reads the alias region that this memory operation works with.
    pub const fn alias_region(self) -> Option<AliasRegion> {
        AliasRegion::from_bits(
            self.read_state(WEIGHT_ALIAS_REGION, WEIGHT_ORDERING)
        )
    }

    /// Sets the alias region that this works on to the specified `region`.
    pub const fn with_alias_region(mut self, region: Option<AliasRegion>) -> Self {
        let bits = AliasRegion::to_bits(region);
        
        self.with_state(bits, WEIGHT_ALIAS_REGION, WEIGHT_ORDERING)
    }

    /// Sets the alias region that this works on to the specified `region`.
    pub fn set_alias_region(&mut self, region: Option<AliasRegion>) {
        *self = self.with_alias_region(region);
    }

    /// Set a flag bit by name.
    ///
    /// Returns true if the flag was found and set, false for an unknown flag
    /// name.
    ///
    /// # Errors
    ///
    /// Returns an error message if the `name` is known but couldn't be applied
    /// due to it being a semantic error.
    pub fn set_by_name(&mut self, name: &str) -> Result<bool, &'static str> {
        *self = match name {
            "notrap" => self.with_trap_code(None),
            "aligned" => self.with_aligned(),
            "little" => {
                if self.read_bit(BIT_BIG_ENDIAN) {
                    return Err("cannot set both big and little endian bits");
                }
                self.with_endianness(Endianness::Little)
            }
            "big" => {
                if self.read_bit(BIT_LITTLE_ENDIAN) {
                    return Err("cannot set both big and little endian bits");
                }
                self.with_endianness(Endianness::Big)
            }
            "heap" => {
                if self.alias_region().is_some() {
                    return Err("cannot set more than one alias region");
                }
                self.with_alias_region(Some(AliasRegion::Heap))
            }
            "table" => {
                if self.alias_region().is_some() {
                    return Err("cannot set more than one alias region");
                }
                self.with_alias_region(Some(AliasRegion::Table))
            }
            "vmctx" => {
                if self.alias_region().is_some() {
                    return Err("cannot set more than one alias region");
                }
                self.with_alias_region(Some(AliasRegion::Vmctx))
            }
            "checked" => self.with_checked(),

            other => match TrapCode::from_str(other) {
                Ok(code) => self.with_trap_code(Some(code)),
                Err(()) => return Ok(false),
            },
        };
        Ok(true)
    }

    /// Express this struct as a standard [MemFlags]
    pub fn as_memflags(self) -> MemFlags {
        let mut flags = MemFlags::new();

        if self.aligned() {
            flags.set_aligned();
        }
        match self.explicit_endianness() {
            Some(Endianness::Little) => flags.set_endianness(Endianness::Little),
            Some(Endianness::Big) => flags.set_endianness(Endianness::Big),
            None => {}
        }
        if self.checked() {
            flags.set_checked();
        }

        flags.set_alias_region(self.alias_region());

        flags = flags.with_trap_code(self.trap_code());

        flags
    }

    /// Gets the [AtomicOrdering] of this operation
    pub const fn atomic_ordering(self) -> AtomicOrdering {
        AtomicOrdering::from_u8(self.read_state(WEIGHT_ORDERING, WEIGHT_BEFORE_ORDERING))
    }

    /// Sets the [AtomicOrdering] of this operation
    pub fn set_ordering(&mut self, ordering: AtomicOrdering) {
        *self = self.with_ordering(ordering);
    }

    /// Sets the [AtomicOrdering] of this operation
    pub const fn with_ordering(mut self, ordering: AtomicOrdering) -> Self {
        self.with(AtomicOrdering.to_u8(ordering), WEIGHT_ORDERING, WEIGHT_BEFORE_ORDERING)
    }

    /// Return endianness of the memory access.  This will return the endianness
    /// explicitly specified by the flags if any, and will default to the native
    /// endianness otherwise.  The native endianness has to be provided by the
    /// caller since it is not explicitly encoded in CLIF IR -- this allows a
    /// front end to create IR without having to know the target endianness.
    pub const fn endianness(self, native_endianness: Endianness) -> Endianness {
        if self.read_bit(BIT_LITTLE_ENDIAN) {
            Endianness::Little
        } else if self.read_bit(BIT_BIG_ENDIAN) {
            Endianness::Big
        } else {
            native_endianness
        }
    }

    /// Return endianness of the memory access, if explicitly specified.
    ///
    /// If the endianness is not explicitly specified, this will return `None`,
    /// which means "native endianness".
    pub const fn explicit_endianness(self) -> Option<Endianness> {
        if self.read_bit(BIT_LITTLE_ENDIAN) {
            Some(Endianness::Little)
        } else if self.read_bit(BIT_BIG_ENDIAN) {
            Some(Endianness::Big)
        } else {
            None
        }
    }

    /// Set endianness of the memory access.
    pub fn set_endianness(&mut self, endianness: Endianness) {
        *self = self.with_endianness(endianness);
    }

    /// Set endianness of the memory access, returning new flags.
    pub const fn with_endianness(self, endianness: Endianness) -> Self {
        let res = match endianness {
            Endianness::Little => self.with_bit(BIT_LITTLE_ENDIAN),
            Endianness::Big => self.with_bit(BIT_BIG_ENDIAN),
        };
        assert!(!(res.read_bit(BIT_LITTLE_ENDIAN) && res.read_bit(BIT_BIG_ENDIAN)));
        res
    }

    /// Test if this memory operation cannot trap.
    ///
    /// By default `AtomicCasMemFlags` will assume that any load/store can trap and is
    /// associated with a `TrapCode::HeapOutOfBounds` code. If the trap code is
    /// configured to `None` though then this method will return `true` and
    /// indicates that the memory operation will not trap.
    ///
    /// If this returns `true` then the memory is *accessible*, which means
    /// that accesses will not trap. This makes it possible to delete an unused
    /// load or a dead store instruction.
    ///
    /// This flag does *not* mean that the associated instruction can be
    /// code-motioned to arbitrary places in the function so long as its data
    /// dependencies are met. This only means that, given its current location
    /// in the function, it will never trap. See the `can_move` method for more
    /// details.
    pub const fn notrap(self) -> bool {
        self.trap_code().is_none()
    }

    /// Sets the trap code for this `AtomicCasMemFlags` to `None`.
    pub fn set_notrap(&mut self) {
        *self = self.with_notrap();
    }

    /// Sets the trap code for this `AtomicCasMemFlags` to `None`, returning the new
    /// flags.
    pub const fn with_notrap(self) -> Self {
        self.with_trap_code(None)
    }

    /// Test if the `aligned` flag is set.
    ///
    /// By default, Cranelift memory instructions work with any unaligned effective address. If the
    /// `aligned` flag is set, the instruction is permitted to trap or return a wrong result if the
    /// effective address is misaligned.
    pub const fn aligned(self) -> bool {
        self.read_bit(BIT_ALIGNED)
    }

    /// Set the `aligned` flag.
    pub fn set_aligned(&mut self) {
        *self = self.with_aligned();
    }

    /// Set the `aligned` flag, returning new flags.
    pub const fn with_aligned(self) -> Self {
        self.with_bit(BIT_ALIGNED)
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
        self.read_bit(BIT_CHECKED)
    }

    /// Set the `checked` bit.
    pub fn set_checked(&mut self) {
        *self = self.with_checked();
    }

    /// Set the `checked` bit, returning new flags.
    pub const fn with_checked(self) -> Self {
        self.with_bit(BIT_CHECKED)
    }

    /// Get the trap code to report if this memory access traps.
    ///
    /// A `None` trap code indicates that this memory access does not trap.
    pub const fn trap_code(self) -> Option<TrapCode> {
        let byte = ((self.bits & MASK_TRAP_CODE) >> TRAP_CODE_OFFSET) as u8;
        match NonZeroU8::new(byte) {
            Some(code) => Some(TrapCode::from_raw(code)),
            None => None,
        }
    }

    /// Configures these flags with the specified trap code `code`.
    ///
    /// A trap code indicates that this memory operation cannot be optimized
    /// away and it must "stay where it is" in the programs. Traps are
    /// considered side effects, for example, and have meaning through the trap
    /// code that is communicated and which instruction trapped.
    pub const fn with_trap_code(mut self, code: Option<TrapCode>) -> Self {
        let bits = match code {
            Some(code) => code.as_raw().get() as u16,
            None => 0,
        };
        self.bits &= !MASK_TRAP_CODE;
        self.bits |= bits << TRAP_CODE_OFFSET;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_traps() {
        for ordering in AtomicOrdering::all() {
            for trap in TrapCode::non_user_traps().iter().copied() {
                let flags = AtomicCasMemFlags::new(*ordering).with_trap_code(Some(trap));
                assert_eq!(flags.trap_code(), Some(trap));
            }

            let flags = AtomicCasMemFlags::new(*ordering).with_trap_code(None);
            assert_eq!(flags.trap_code(), None);
        }
    }

    #[test]
    fn cannot_set_big_and_little() {
        for ordering in AtomicOrdering::all() {
            let mut big = AtomicCasMemFlags::new(*ordering).with_endianness(Endianness::Big);
            assert!(big.set_by_name("little").is_err());

            let mut little = AtomicCasMemFlags::new(*ordering).with_endianness(Endianness::Little);
            assert!(little.set_by_name("big").is_err());
        }
    }

    #[test]
    fn only_one_region() {
        for ordering in AtomicOrdering::all() {
            let mut big =
                AtomicCasMemFlags::new(*ordering).with_alias_region(Some(AliasRegion::Heap));
            assert!(big.set_by_name("table").is_err());
            assert!(big.set_by_name("vmctx").is_err());

            let mut big =
                AtomicCasMemFlags::new(*ordering).with_alias_region(Some(AliasRegion::Table));
            assert!(big.set_by_name("heap").is_err());
            assert!(big.set_by_name("vmctx").is_err());

            let mut big =
                AtomicCasMemFlags::new(*ordering).with_alias_region(Some(AliasRegion::Vmctx));
            assert!(big.set_by_name("heap").is_err());
            assert!(big.set_by_name("table").is_err());
        }
    }

    #[test]
    fn check_atomic_ordering() {
        for ordering in AtomicOrdering::all() {
            let mut big = AtomicCasMemFlags::new(*ordering);

            assert!(big.get_ordering() == *ordering);

            for ordering in AtomicOrdering::all() {
                big.set_ordering(*ordering);
                assert!(big.get_ordering() == *ordering);
            }
        }
    }
}
