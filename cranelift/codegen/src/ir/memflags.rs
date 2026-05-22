//! Memory operation flags.

use super::TrapCode;
use crate::HashMap;
use crate::entity::{self, PrimaryMap};
pub use crate::machinst::MachMemFlags;
use alloc::borrow::Cow;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Index;
use core::str::FromStr;
use cranelift_entity::{entity_impl, packed_option::PackedOption};

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// Endianness of a memory access.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Endianness {
    /// Little-endian
    Little,
    /// Big-endian
    Big,
}

/// An opaque reference to an alias region.
///
/// Alias regions identify disjoint categories of memory for alias analysis.
/// Two memory operations in different alias regions are known not to alias.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct AliasRegion(u32);
entity_impl!(AliasRegion, "region");

/// Data describing an alias region.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct AliasRegionData {
    /// A unique, user-defined identifier for this alias region.
    ///
    /// Alias regions are deduplicated based on this identifier.
    ///
    /// This deduplication happens during inlining, for example, when a
    /// callee's alias regions are merged with the caller's. Therefore, when
    /// inlining is enabled this identifier should be globally unique across
    /// the whole compilation. When inlining is disabled, it is sufficient
    /// to be unique within the context of a single function.
    pub user_id: u32,

    /// Description of this alias region, e.g. "vmctx", "funcref table",
    /// "global 42", or "gc struct `LinkedList` field `tail`".
    ///
    /// This only exists for printing in the CLIF text format.
    pub description: Cow<'static, str>,
}

/// An opaque reference to memory operation flags stored in a
/// [`MemFlagsSet`].
///
/// `MemFlags` is a u16 entity index that refers to a [`MemFlagsData`] entry in
/// the [`MemFlagsSet`] stored in the
/// [`DataFlowGraph`](super::dfg::DataFlowGraph).
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemFlags(u16);

impl MemFlags {
    /// Create a new `MemFlags` from a `u32` index.
    ///
    /// Returns `None` if the index doesn't fit in a `u16`.
    pub fn with_number(n: u32) -> Option<Self> {
        let val = u16::try_from(n).ok()?;
        if val == u16::MAX {
            None
        } else {
            Some(Self(val))
        }
    }
}

impl entity::EntityRef for MemFlags {
    #[inline]
    fn new(index: usize) -> Self {
        let val = u16::try_from(index).expect("MemFlags index overflow");
        Self(val)
    }

    #[inline]
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl entity::packed_option::ReservedValue for MemFlags {
    #[inline]
    fn reserved_value() -> Self {
        Self(u16::MAX)
    }

    #[inline]
    fn is_reserved_value(&self) -> bool {
        self.0 == u16::MAX
    }
}

impl MemFlags {
    /// Create a new instance from a `u32`.
    #[inline]
    pub fn from_u32(x: u32) -> Self {
        Self(u16::try_from(x).unwrap())
    }

    /// Return the underlying index value as a `u32`.
    #[inline]
    pub fn as_u32(self) -> u32 {
        u32::from(self.0)
    }

    /// Return the raw bit encoding for this instance.
    #[inline]
    pub fn as_bits(self) -> u32 {
        u32::from(self.0)
    }

    /// Create a new instance from the raw bit encoding.
    #[inline]
    pub fn from_bits(x: u32) -> Self {
        Self(u16::try_from(x).unwrap())
    }
}

impl fmt::Display for MemFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "memflags{}", self.0)
    }
}

impl fmt::Debug for MemFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self as &dyn fmt::Display).fmt(f)
    }
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
pub struct MemFlagsData {
    /// Backend-facing memory-operation flags.
    flags: MachMemFlags,

    /// The alias region for this memory operation, if any.
    region: PackedOption<AliasRegion>,
}

const fn no_alias_region() -> PackedOption<AliasRegion> {
    PackedOption::new(AliasRegion(u32::MAX))
}

impl MemFlagsData {
    /// Create a new empty set of flags.
    pub const fn new() -> Self {
        Self {
            flags: MachMemFlags::new(),
            region: no_alias_region(),
        }
    }

    /// Create a set of flags representing an access from a "trusted" address, meaning it's
    /// known to be aligned and non-trapping.
    pub const fn trusted() -> Self {
        Self::new().with_notrap().with_aligned()
    }

    /// Reads the alias region that this memory operation works with.
    pub fn alias_region(self) -> Option<AliasRegion> {
        self.region.expand()
    }

    /// Sets the alias region that this works on to the specified `region`.
    pub fn with_alias_region(mut self, region: Option<AliasRegion>) -> Self {
        self.region = region.into();
        self
    }

    /// Sets the alias region that this works on to the specified `region`.
    pub fn set_alias_region(&mut self, region: Option<AliasRegion>) {
        self.region = region.into();
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
            "readonly" => self.with_readonly(),
            "little" => {
                if self.flags.explicit_endianness() == Some(Endianness::Big) {
                    return Err("cannot set both big and little endian bits");
                }
                self.with_endianness(Endianness::Little)
            }
            "big" => {
                if self.flags.explicit_endianness() == Some(Endianness::Little) {
                    return Err("cannot set both big and little endian bits");
                }
                self.with_endianness(Endianness::Big)
            }
            "can_move" => self.with_can_move(),

            other => match TrapCode::from_str(other) {
                Ok(code) => self.with_trap_code(Some(code)),
                Err(()) => return Ok(false),
            },
        };
        Ok(true)
    }

    /// Return endianness of the memory access.  This will return the endianness
    /// explicitly specified by the flags if any, and will default to the native
    /// endianness otherwise.  The native endianness has to be provided by the
    /// caller since it is not explicitly encoded in CLIF IR -- this allows a
    /// front end to create IR without having to know the target endianness.
    pub const fn endianness(self, native_endianness: Endianness) -> Endianness {
        self.flags.endianness(native_endianness)
    }

    /// Return endianness of the memory access, if explicitly specified.
    ///
    /// If the endianness is not explicitly specified, this will return `None`,
    /// which means "native endianness".
    pub const fn explicit_endianness(self) -> Option<Endianness> {
        self.flags.explicit_endianness()
    }

    /// Set endianness of the memory access.
    pub fn set_endianness(&mut self, endianness: Endianness) {
        *self = self.with_endianness(endianness);
    }

    /// Set endianness of the memory access, returning new flags.
    pub const fn with_endianness(mut self, endianness: Endianness) -> Self {
        self.flags = self.flags.with_endianness(endianness);
        self
    }

    /// Test if this memory operation cannot trap.
    ///
    /// By default `MemFlags` will assume that any load/store can trap and is
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

    /// Sets the trap code for this `MemFlagsData` to `None`.
    pub fn set_notrap(&mut self) {
        *self = self.with_notrap();
    }

    /// Sets the trap code for this `MemFlagsData` to `None`, returning the new
    /// flags.
    pub const fn with_notrap(self) -> Self {
        self.with_trap_code(None)
    }

    /// Is this memory operation safe to move so long as its data dependencies
    /// remain satisfied?
    ///
    /// If this is `true`, then it is okay to code motion this instruction to
    /// arbitrary locations, in the function, including across blocks and
    /// conditional branches, so long as data dependencies (and trap ordering,
    /// if any) are upheld.
    ///
    /// If this is `false`, then this memory operation's safety potentially
    /// relies upon invariants that are not reflected in its data dependencies,
    /// and therefore it is not safe to code motion this operation. For example,
    /// this operation could be in a block that is dominated by a control-flow
    /// bounds check, which is not reflected in its operands, and it would be
    /// unsafe to code motion it above the bounds check, even if its data
    /// dependencies would still be satisfied.
    pub const fn can_move(self) -> bool {
        self.flags.can_move()
    }

    /// Set the `can_move` flag.
    pub const fn set_can_move(&mut self) {
        *self = self.with_can_move();
    }

    /// Set the `can_move` flag, returning new flags.
    pub const fn with_can_move(mut self) -> Self {
        self.flags = self.flags.with_can_move();
        self
    }

    /// Test if the `aligned` flag is set.
    ///
    /// By default, Cranelift memory instructions work with any unaligned effective address. If the
    /// `aligned` flag is set, the instruction is permitted to trap or return a wrong result if the
    /// effective address is misaligned.
    pub const fn aligned(self) -> bool {
        self.flags.aligned()
    }

    /// Set the `aligned` flag.
    pub fn set_aligned(&mut self) {
        *self = self.with_aligned();
    }

    /// Set the `aligned` flag, returning new flags.
    pub const fn with_aligned(mut self) -> Self {
        self.flags = self.flags.with_aligned();
        self
    }

    /// Test if the `readonly` flag is set.
    ///
    /// Loads with this flag have no memory dependencies.
    /// This results in undefined behavior if the dereferenced memory is mutated at any time
    /// between when the function is called and when it is exited.
    pub const fn readonly(self) -> bool {
        self.flags.readonly()
    }

    /// Set the `readonly` flag.
    pub fn set_readonly(&mut self) {
        *self = self.with_readonly();
    }

    /// Set the `readonly` flag, returning new flags.
    pub const fn with_readonly(mut self) -> Self {
        self.flags = self.flags.with_readonly();
        self
    }
    /// Get the trap code to report if this memory access traps.
    ///
    /// A `None` trap code indicates that this memory access does not trap.
    pub const fn trap_code(self) -> Option<TrapCode> {
        self.flags.trap_code()
    }

    /// Configures these flags with the specified trap code `code`.
    ///
    /// A trap code indicates that this memory operation cannot be optimized
    /// away and it must "stay where it is" in the programs. Traps are
    /// considered side effects, for example, and have meaning through the trap
    /// code that is communicated and which instruction trapped.
    pub const fn with_trap_code(mut self, code: Option<TrapCode>) -> Self {
        self.flags = self.flags.with_trap_code(code);
        self
    }
}

impl From<MemFlagsData> for MachMemFlags {
    fn from(flags: MemFlagsData) -> Self {
        flags.flags
    }
}

impl From<MachMemFlags> for MemFlagsData {
    fn from(flags: MachMemFlags) -> Self {
        Self {
            flags,
            region: no_alias_region(),
        }
    }
}

impl fmt::Display for MemFlagsData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.flags)?;
        match self.alias_region() {
            None => {}
            Some(region) => write!(f, " {region}")?,
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemFlagsSetOverflow;

/// A deduplicated set of mem flags.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemFlagsSet {
    mem_flags: PrimaryMap<MemFlags, MemFlagsData>,
    dedupe_map: HashMap<MemFlagsData, MemFlags>,
}

impl PartialEq for MemFlagsSet {
    fn eq(&self, other: &Self) -> bool {
        self.mem_flags == other.mem_flags
    }
}

impl Eq for MemFlagsSet {}

impl Hash for MemFlagsSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mem_flags.hash(state);
    }
}

impl MemFlagsSet {
    /// Create a new empty set.
    pub fn new() -> Self {
        Self {
            mem_flags: PrimaryMap::new(),
            dedupe_map: HashMap::new(),
        }
    }

    /// Insert new mem flags into this set.
    ///
    /// Returns an existing `MemFlags` if the data already exists.
    pub fn insert(&mut self, data: MemFlagsData) -> Result<MemFlags, MemFlagsSetOverflow> {
        if let Some(&existing) = self.dedupe_map.get(&data) {
            return Ok(existing);
        }
        let next = u32::try_from(self.mem_flags.len())
            .ok()
            .and_then(MemFlags::with_number)
            .ok_or(MemFlagsSetOverflow)?;
        let key = self.mem_flags.push(data);
        debug_assert_eq!(key, next);
        self.dedupe_map.insert(data, key);
        Ok(key)
    }

    /// Insert new mem flags into this set, panicking if the index does not fit.
    pub fn insert_unchecked(&mut self, data: MemFlagsData) -> MemFlags {
        match self.insert(data) {
            Ok(flags) => flags,
            Err(_) => panic!("MemFlags index overflow"),
        }
    }

    /// Returns `true` if the given mem flags reference is valid.
    pub fn is_valid(&self, mf: MemFlags) -> bool {
        self.mem_flags.is_valid(mf)
    }

    /// Clear the set.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Find the entity index for an existing [`MemFlagsData`] value.
    pub fn get(&self, data: MemFlagsData) -> Option<MemFlags> {
        self.dedupe_map.get(&data).copied()
    }
}

// NB: Do not implement `IndexMut` because mem flags data is deduped and shared
// by many instructions.
impl Index<MemFlags> for MemFlagsSet {
    type Output = MemFlagsData;

    fn index(&self, mf: MemFlags) -> &MemFlagsData {
        &self.mem_flags[mf]
    }
}

/// A deduplicated set of alias regions.
///
/// Deduplication is based on `user_id`; the description string is not
/// considered.
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct AliasRegionSet {
    alias_regions: PrimaryMap<AliasRegion, AliasRegionData>,
    dedupe_map: HashMap<u32, AliasRegion>,
}

impl Hash for AliasRegionSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.alias_regions.hash(state);
    }
}

impl AliasRegionSet {
    /// Create a new empty set.
    pub fn new() -> Self {
        Self {
            alias_regions: PrimaryMap::new(),
            dedupe_map: HashMap::new(),
        }
    }

    /// Insert a new alias region into this set.
    ///
    /// Returns an existing `AliasRegion` if one with the same `user_id`
    /// already exists.
    pub fn insert(&mut self, data: AliasRegionData) -> AliasRegion {
        if let Some(&existing) = self.dedupe_map.get(&data.user_id) {
            return existing;
        }
        let user_id = data.user_id;
        let key = self.alias_regions.push(data);
        self.dedupe_map.insert(user_id, key);
        key
    }

    /// Push a new alias region, bypassing deduplication.
    ///
    /// This is used by the CLIF text parser to faithfully represent the
    /// source text. The verifier will then check for duplicate `user_id`s.
    pub fn push(&mut self, data: AliasRegionData) -> AliasRegion {
        let user_id = data.user_id;
        let key = self.alias_regions.push(data);
        self.dedupe_map.insert(user_id, key);
        key
    }

    /// Returns `true` if this set already contains a region with the given
    /// `user_id`.
    pub fn contains(&self, user_id: u32) -> bool {
        self.dedupe_map.contains_key(&user_id)
    }

    /// Returns `true` if the given alias region reference is valid.
    pub fn is_valid(&self, ar: AliasRegion) -> bool {
        self.alias_regions.is_valid(ar)
    }

    /// Return the number of alias regions in the set.
    pub fn len(&self) -> usize {
        self.alias_regions.len()
    }

    /// Iterate over all alias regions and their data.
    pub fn iter(&self) -> impl Iterator<Item = (AliasRegion, &AliasRegionData)> {
        self.alias_regions.iter()
    }

    /// Clear the set.
    pub fn clear(&mut self) {
        self.alias_regions.clear();
        self.dedupe_map.clear();
    }
}

// NB: Do not implement `IndexMut` because alias region data is deduped and
// shared by many mem flags.
impl Index<AliasRegion> for AliasRegionSet {
    type Output = AliasRegionData;

    fn index(&self, ar: AliasRegion) -> &AliasRegionData {
        &self.alias_regions[ar]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_entity::EntityRef;

    #[test]
    fn roundtrip_traps() {
        for trap in TrapCode::non_user_traps().iter().copied() {
            let _flags = MemFlagsData::new().with_trap_code(Some(trap));
        }
        let _flags = MemFlagsData::new().with_trap_code(None);
    }

    #[test]
    fn cannot_set_big_and_little() {
        let _big = MemFlagsData::new().with_endianness(Endianness::Big);

        let _little = MemFlagsData::new().with_endianness(Endianness::Little);
    }

    #[test]
    fn only_one_region() {
        let region0 = AliasRegion::new(0);
        let region1 = AliasRegion::new(1);
        let flags = MemFlagsData::new().with_alias_region(Some(region0));
        assert_eq!(flags.alias_region(), Some(region0));

        let flags = flags.with_alias_region(Some(region1));
        assert_eq!(flags.alias_region(), Some(region1));

        let flags = flags.with_alias_region(None);
        assert_eq!(flags.alias_region(), None);
    }

    #[test]
    fn clear_removes_entries() {
        let mut set = MemFlagsSet::new();
        let trusted = set.insert(MemFlagsData::trusted()).unwrap();
        let custom = MemFlagsData::new()
            .with_endianness(Endianness::Big)
            .with_alias_region(Some(AliasRegion::new(0)));
        let custom_key = set.insert(custom).unwrap();
        assert!(set.is_valid(trusted));
        assert!(set.is_valid(custom_key));

        set.clear();

        assert!(!set.is_valid(trusted));
        assert!(!set.is_valid(custom_key));
        let trusted = set.insert(MemFlagsData::trusted()).unwrap();
        assert_eq!(set[trusted], MemFlagsData::trusted());
    }
}
