//! Alias regions: disjoint categories of memory for alias analysis.

use crate::HashMap;
use crate::entity::PrimaryMap;
use alloc::borrow::Cow;
use core::hash::{Hash, Hasher};
use core::ops::Index;
use cranelift_entity::{entity_impl, packed_option::PackedOption};

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// An opaque reference to an alias region.
///
/// Alias regions identify disjoint categories of memory for alias analysis.
/// Two memory operations in different alias regions are known not to alias.
/// An access with no region occupies a separate implicit "other" category,
/// also disjoint from every named region (explicit no-region).
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

/// The packed representation of "no alias region".
pub(crate) const fn no_alias_region() -> PackedOption<AliasRegion> {
    PackedOption::new(AliasRegion(u32::MAX))
}

/// A deduplicated table of alias region declarations.
///
/// Deduplication is based on `user_id`; the description string is not
/// considered.
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct AliasRegionTable {
    alias_regions: PrimaryMap<AliasRegion, AliasRegionData>,
    dedupe_map: HashMap<u32, AliasRegion>,
}

impl Hash for AliasRegionTable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.alias_regions.hash(state);
    }
}

impl AliasRegionTable {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self {
            alias_regions: PrimaryMap::new(),
            dedupe_map: HashMap::new(),
        }
    }

    /// Insert a new alias region into this table.
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

    /// Returns `true` if this table already contains a region with the given
    /// `user_id`.
    pub fn contains(&self, user_id: u32) -> bool {
        self.dedupe_map.contains_key(&user_id)
    }

    /// Get the alias region with the given id, if any.
    pub fn get(&self, user_id: u32) -> Option<AliasRegion> {
        self.dedupe_map.get(&user_id).copied()
    }

    /// Returns `true` if the given alias region reference is valid.
    pub fn is_valid(&self, ar: AliasRegion) -> bool {
        self.alias_regions.is_valid(ar)
    }

    /// Return the number of alias regions in the table.
    pub fn len(&self) -> usize {
        self.alias_regions.len()
    }

    /// Iterate over all alias regions and their data.
    pub fn iter(&self) -> impl Iterator<Item = (AliasRegion, &AliasRegionData)> {
        self.alias_regions.iter()
    }

    /// Clear the table.
    pub fn clear(&mut self) {
        self.alias_regions.clear();
        self.dedupe_map.clear();
    }
}

// NB: Do not implement `IndexMut` because alias region data is deduped and
// shared by many mem flags.
impl Index<AliasRegion> for AliasRegionTable {
    type Output = AliasRegionData;

    fn index(&self, ar: AliasRegion) -> &AliasRegionData {
        &self.alias_regions[ar]
    }
}
