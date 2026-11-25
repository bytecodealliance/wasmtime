//! Alias regions: disjoint categories of memory for alias analysis.

use crate::HashMap;
use crate::entity::{EntityRef, PrimaryMap, SmallEntitySet, SmallSetIter};
use crate::ir::DataFlowGraph;
use alloc::borrow::Cow;
use core::fmt;
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

/// A set of alias regions, possibly including the anonymous "other" region
/// that accesses without an alias region use.
///
/// Sets can be represented either by the regions they include or by the
/// regions they exclude, so that "all regions" and "all regions except `A` and
/// `B`" are as cheap as "just `A` and `B`". The excluding form is relative to
/// whatever regions a function declares.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct AliasRegionSet {
    /// The named regions in this set.
    named_regions: AliasRegionSetInner,

    /// Whether the "other" anonymous region is in this set.
    other: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
enum AliasRegionSetInner {
    /// Just these named regions are in this set.
    Including(SmallEntitySet<AliasRegion>),

    /// All named regions except for these are in this set.
    Excluding(SmallEntitySet<AliasRegion>),
}

impl AliasRegionSet {
    /// Create a set of just the given regions, where `None` is the "other"
    /// anonymous region.
    pub fn new(regions: impl IntoIterator<Item = Option<AliasRegion>>) -> Self {
        let mut named = SmallEntitySet::new();
        let mut other = false;
        for region in regions {
            match region {
                Some(r) => {
                    named.insert(r);
                }
                None => other = true,
            }
        }
        Self {
            named_regions: AliasRegionSetInner::Including(named),
            other,
        }
    }

    /// Create a set of all regions except the given ones, where `None` is the
    /// "other" anonymous region.
    pub fn new_excluding(regions: impl IntoIterator<Item = Option<AliasRegion>>) -> Self {
        let mut named = SmallEntitySet::new();
        let mut other = true;
        for region in regions {
            match region {
                Some(r) => {
                    named.insert(r);
                }
                None => other = false,
            }
        }
        Self {
            named_regions: AliasRegionSetInner::Excluding(named),
            other,
        }
    }

    /// Create the set of all regions.
    pub fn all() -> Self {
        Self::new_excluding([])
    }

    /// Create the empty set.
    pub fn none() -> Self {
        Self::new([])
    }

    /// Is the given region in this set? `None` is the "other" anonymous region.
    #[inline]
    pub fn contains(&self, region: Option<AliasRegion>) -> bool {
        match region {
            None => self.other,
            Some(r) => match &self.named_regions {
                AliasRegionSetInner::Including(set) => set.contains(r),
                AliasRegionSetInner::Excluding(set) => !set.contains(r),
            },
        }
    }

    /// Is this the set of all regions?
    #[inline]
    pub fn is_all(&self) -> bool {
        self.other
            && matches!(&self.named_regions, AliasRegionSetInner::Excluding(set) if set.is_empty())
    }

    /// Is this the empty set?
    #[inline]
    pub fn is_none(&self) -> bool {
        !self.other
            && matches!(&self.named_regions, AliasRegionSetInner::Including(set) if set.is_empty())
    }

    /// Does this set contain every named region that it does not explicitly
    /// list?
    ///
    /// This is true for sets created with `new_excluding` (and `all`), and
    /// answers whether a region that has not been declared yet is in the set.
    #[inline]
    pub fn contains_unnamed(&self) -> bool {
        matches!(self.named_regions, AliasRegionSetInner::Excluding(_))
    }

    /// Create a new set by mapping each of this set's explicitly listed named
    /// regions through `f`.
    pub fn map(&self, mut f: impl FnMut(AliasRegion) -> AliasRegion) -> Self {
        let named_regions = match &self.named_regions {
            AliasRegionSetInner::Including(set) => {
                AliasRegionSetInner::Including(set.iter().map(&mut f).collect())
            }
            AliasRegionSetInner::Excluding(set) => {
                AliasRegionSetInner::Excluding(set.iter().map(&mut f).collect())
            }
        };
        Self {
            named_regions,
            other: self.other,
        }
    }

    /// Iterate over the regions in this set, relative to the regions declared
    /// in `dfg`.
    ///
    /// Yields the "other" region as `None` first, if present, and then the
    /// named regions in ascending order.
    pub fn iter<'a>(&'a self, dfg: &DataFlowGraph) -> AliasRegionSetIter<'a> {
        let named = match &self.named_regions {
            AliasRegionSetInner::Including(set) => NamedIter::Including(set.iter(), set.len()),
            AliasRegionSetInner::Excluding(set) => NamedIter::excluding(dfg, Some(set)),
        };
        AliasRegionSetIter::new(self.other, named)
    }
}

impl fmt::Display for AliasRegionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (named, other_listed) = match &self.named_regions {
            AliasRegionSetInner::Including(set) => {
                if set.is_empty() && !self.other {
                    return write!(f, "none");
                }
                (set, self.other)
            }
            AliasRegionSetInner::Excluding(set) => {
                if set.is_empty() && self.other {
                    return write!(f, "all");
                }
                write!(f, "all except ")?;
                (set, !self.other)
            }
        };
        let mut sep = "";
        for region in named.iter() {
            write!(f, "{sep}{region}")?;
            sep = ", ";
        }
        if other_listed {
            write!(f, "{sep}other")?;
        }
        Ok(())
    }
}

/// A reference to a set of alias regions.
///
/// This allows returning a "reference" to an empty set, a singleton set, or the
/// universe without materializing a whole `AliasRegionSet` that we can take a
/// reference to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasRegionSetRef<'a> {
    /// A reference to the empty set.
    None,

    /// A reference to the singleton set of the given region. `None` is the
    /// "other" anonymous region.
    Just(Option<AliasRegion>),

    /// A reference to the given set of alias regions.
    Set(&'a AliasRegionSet),

    /// A reference to the set of all regions.
    All,
}

impl<'a> From<&'a AliasRegionSet> for AliasRegionSetRef<'a> {
    #[inline]
    fn from(set: &'a AliasRegionSet) -> Self {
        Self::Set(set)
    }
}

impl<'a> AliasRegionSetRef<'a> {
    /// Is the given region in this set? `None` is the "other" anonymous region.
    #[inline]
    pub fn contains(&self, region: Option<AliasRegion>) -> bool {
        match self {
            Self::None => false,
            Self::Just(r) => *r == region,
            Self::Set(set) => set.contains(region),
            Self::All => true,
        }
    }

    /// Is this the set of all regions?
    #[inline]
    pub fn is_all(&self) -> bool {
        match self {
            Self::All => true,
            Self::Set(set) => set.is_all(),
            Self::None | Self::Just(_) => false,
        }
    }

    /// Is this the empty set?
    #[inline]
    pub fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            Self::Set(set) => set.is_none(),
            Self::Just(_) | Self::All => false,
        }
    }

    /// Iterate over the regions in this set, relative to the regions declared
    /// in `dfg`.
    ///
    /// Yields the "other" region as `None` first, if present, and then the
    /// named regions in ascending order.
    pub fn iter(&self, dfg: &DataFlowGraph) -> AliasRegionSetIter<'a> {
        match *self {
            Self::None => AliasRegionSetIter::new(false, NamedIter::Just(None)),
            Self::Just(r) => AliasRegionSetIter::new(r.is_none(), NamedIter::Just(r)),
            Self::Set(set) => set.iter(dfg),
            Self::All => AliasRegionSetIter::new(true, NamedIter::excluding(dfg, None)),
        }
    }
}

/// An iterator over the regions in an `AliasRegionSet` or `AliasRegionSetRef`.
///
/// Yields `None` for the "other" anonymous region.
pub struct AliasRegionSetIter<'a> {
    /// Whether we have yet to yield the "other" region.
    other: bool,
    named: NamedIter<'a>,
    len: usize,
}

enum NamedIter<'a> {
    Just(Option<AliasRegion>),
    /// An iterator over the given set and its length.
    Including(SmallSetIter<'a, AliasRegion>, usize),
    Excluding {
        next: usize,
        end: usize,
        excluded: Option<&'a SmallEntitySet<AliasRegion>>,
    },
}

impl<'a> NamedIter<'a> {
    fn excluding(dfg: &DataFlowGraph, excluded: Option<&'a SmallEntitySet<AliasRegion>>) -> Self {
        Self::Excluding {
            next: 0,
            end: dfg.alias_regions.len(),
            excluded,
        }
    }

    fn len(&self) -> usize {
        match self {
            NamedIter::Just(r) => usize::from(r.is_some()),
            NamedIter::Including(_, len) => *len,
            NamedIter::Excluding {
                next,
                end,
                excluded,
            } => (*next..*end)
                .filter(|i| !excluded.is_some_and(|set| set.contains(AliasRegion::new(*i))))
                .count(),
        }
    }
}

impl<'a> AliasRegionSetIter<'a> {
    fn new(other: bool, named: NamedIter<'a>) -> Self {
        let len = usize::from(other) + named.len();
        Self { other, named, len }
    }
}

impl Iterator for AliasRegionSetIter<'_> {
    type Item = Option<AliasRegion>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = if core::mem::take(&mut self.other) {
            Some(None)
        } else {
            match &mut self.named {
                NamedIter::Just(r) => r.take().map(Some),
                NamedIter::Including(iter, _) => iter.next().map(Some),
                NamedIter::Excluding {
                    next,
                    end,
                    excluded,
                } => {
                    let mut item = None;
                    while *next < *end {
                        let r = AliasRegion::new(*next);
                        *next += 1;
                        if !excluded.is_some_and(|set| set.contains(r)) {
                            item = Some(Some(r));
                            break;
                        }
                    }
                    item
                }
            }
        };
        if item.is_some() {
            self.len -= 1;
        }
        item
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl ExactSizeIterator for AliasRegionSetIter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::ToString, vec, vec::Vec};

    fn dfg_with_regions(n: u32) -> DataFlowGraph {
        let mut dfg = DataFlowGraph::new();
        for user_id in 0..n {
            dfg.alias_regions.push(AliasRegionData {
                user_id,
                description: "".into(),
            });
        }
        dfg
    }

    fn r(i: usize) -> Option<AliasRegion> {
        Some(AliasRegion::new(i))
    }

    fn check_iter(iter: AliasRegionSetIter<'_>, expected: &[Option<AliasRegion>]) {
        assert_eq!(iter.len(), expected.len());
        assert_eq!(iter.collect::<Vec<_>>(), expected);
    }

    #[test]
    fn set_contains() {
        let s = AliasRegionSet::new([r(1), None]);
        assert!(s.contains(r(1)) && s.contains(None) && !s.contains(r(0)));
        let s = AliasRegionSet::new_excluding([r(1)]);
        assert!(!s.contains(r(1)) && s.contains(None) && s.contains(r(7)));
        let s = AliasRegionSet::new_excluding([None]);
        assert!(!s.contains(None) && s.contains(r(0)));
    }

    #[test]
    fn set_all_none() {
        assert!(AliasRegionSet::all().is_all());
        assert!(!AliasRegionSet::all().is_none());
        assert!(AliasRegionSet::none().is_none());
        assert!(!AliasRegionSet::none().is_all());
        assert!(!AliasRegionSet::new_excluding([None]).is_all());
        assert!(!AliasRegionSet::new([None]).is_none());
        assert!(AliasRegionSet::all().contains_unnamed());
        assert!(!AliasRegionSet::new([r(0)]).contains_unnamed());
    }

    #[test]
    fn set_iter() {
        let dfg = dfg_with_regions(4);
        check_iter(AliasRegionSet::none().iter(&dfg), &[]);
        check_iter(
            AliasRegionSet::all().iter(&dfg),
            &[None, r(0), r(1), r(2), r(3)],
        );
        check_iter(
            AliasRegionSet::new([r(2), None, r(0)]).iter(&dfg),
            &[None, r(0), r(2)],
        );
        check_iter(
            AliasRegionSet::new_excluding([r(1), None, r(9)]).iter(&dfg),
            &[r(0), r(2), r(3)],
        );
    }

    #[test]
    fn set_map() {
        let s =
            AliasRegionSet::new_excluding([r(0), None]).map(|r| AliasRegion::new(r.index() + 1));
        assert_eq!(s, AliasRegionSet::new_excluding([r(1), None]));
    }

    #[test]
    fn set_display() {
        let cases = [
            (AliasRegionSet::none(), "none"),
            (AliasRegionSet::all(), "all"),
            (AliasRegionSet::new([None]), "other"),
            (
                AliasRegionSet::new([r(1), None, r(0)]),
                "region0, region1, other",
            ),
            (AliasRegionSet::new_excluding([None]), "all except other"),
            (
                AliasRegionSet::new_excluding([r(1), None]),
                "all except region1, other",
            ),
        ];
        for (set, expected) in cases {
            assert_eq!(set.to_string(), expected);
        }
    }

    #[test]
    fn set_ref() {
        let dfg = dfg_with_regions(3);
        let set = AliasRegionSet::new([r(1)]);
        let cases = [
            (AliasRegionSetRef::None, vec![]),
            (AliasRegionSetRef::Just(None), vec![None]),
            (AliasRegionSetRef::Just(r(2)), vec![r(2)]),
            (AliasRegionSetRef::from(&set), vec![r(1)]),
            (AliasRegionSetRef::All, vec![None, r(0), r(1), r(2)]),
        ];
        for (set_ref, expected) in cases {
            check_iter(set_ref.iter(&dfg), &expected);
            for region in [None, r(0), r(1), r(2)] {
                assert_eq!(set_ref.contains(region), expected.contains(&region));
            }
            assert_eq!(set_ref.is_all(), expected.len() == 4);
            assert_eq!(set_ref.is_none(), expected.is_empty());
        }
    }
}
