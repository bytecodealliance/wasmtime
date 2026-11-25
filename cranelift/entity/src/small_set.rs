//! A set of entity references, optimized for small sets.

use crate::EntityRef;
use crate::keys::Keys;
use crate::packed_option::ReservedValue;
use crate::set::{EntitySet, SetIter};
use core::fmt;
use core::hash::{Hash, Hasher};
use wasmtime_core::error::OutOfMemory;

/// A set of `K` entity references that stores up to `N` elements inline.
///
/// `SmallEntitySet` is to [`EntitySet`] what `SmallVec` is to `Vec`: it keeps
/// small sets inline, without allocating, and transparently switches to an
/// out-of-line `EntitySet` once it holds more than `N` elements.
///
/// The reserved value of `K` (see [`ReservedValue`]) may never be inserted
/// into a `SmallEntitySet`.
///
/// ```
/// use cranelift_entity::{entity_impl, SmallEntitySet};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// struct Entity(u32);
/// entity_impl!(Entity);
///
/// let mut set = SmallEntitySet::<Entity>::new();
/// set.insert(Entity::from_u32(3));
/// set.insert(Entity::from_u32(1));
///
/// let elems: Vec<_> = set.iter().collect();
/// assert_eq!(elems, [Entity::from_u32(1), Entity::from_u32(3)]);
/// ```
#[derive(Clone)]
pub struct SmallEntitySet<K, const N: usize = 5>(SmallEntitySetInner<K, N>)
where
    K: EntityRef + ReservedValue;

#[derive(Clone)]
enum SmallEntitySetInner<K, const N: usize>
where
    K: EntityRef + ReservedValue,
{
    /// Inline elements for small sets.
    ///
    /// Kept sorted by index, with reserved values (empty slots) last.
    Small([K; N]),

    /// Out-of-line elements for larger sets.
    Set(EntitySet<K>),
}

impl<K, const N: usize> Default for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    #[inline]
    fn default() -> Self {
        Self(SmallEntitySetInner::Small([K::reserved_value(); N]))
    }
}

impl<K, const N: usize> fmt::Debug for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<K, const N: usize> PartialEq for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    fn eq(&self, other: &Self) -> bool {
        // Compare elements, not representations: a spilled set can hold as few
        // elements as an inline one.
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<K, const N: usize> Eq for SmallEntitySet<K, N> where K: EntityRef + ReservedValue {}

impl<K, const N: usize> Hash for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash elements, not representations, to agree with `PartialEq`.
        state.write_usize(self.len());
        for k in self.iter() {
            state.write_usize(k.index());
        }
    }
}

impl<K, const N: usize> Extend<K> for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    #[inline]
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        for k in iter {
            self.insert(k);
        }
    }
}

impl<K, const N: usize> FromIterator<K> for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    #[inline]
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend(iter);
        set
    }
}

impl<'a, K, const N: usize> IntoIterator for &'a SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    type Item = K;
    type IntoIter = SmallSetIter<'a, K>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(feature = "enable-serde")]
impl<K, const N: usize> serde::Serialize for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue + serde::Serialize,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(feature = "enable-serde")]
impl<'de, K, const N: usize> serde::Deserialize<'de> for SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue + serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let elems = alloc::vec::Vec::<K>::deserialize(deserializer)?;
        if elems.iter().any(|k| k.is_reserved_value()) {
            return Err(serde::de::Error::custom(
                "cannot insert the reserved value into a `SmallEntitySet`",
            ));
        }
        Ok(elems.into_iter().collect())
    }
}

impl<K, const N: usize> SmallEntitySet<K, N>
where
    K: EntityRef + ReservedValue,
{
    /// Create a new empty set.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new empty set.
    ///
    /// The capacity is only a hint for out-of-line storage, so this does not
    /// allocate.
    #[inline]
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }

    /// Like `with_capacity` but returns an error on allocation failure.
    #[inline]
    pub fn try_with_capacity(_capacity: usize) -> Result<Self, OutOfMemory> {
        Ok(Self::new())
    }

    /// Ensure that out-of-line storage, if any, has enough capacity to hold
    /// `capacity` total elements.
    ///
    /// Does nothing while the set's elements are stored inline.
    #[inline]
    pub fn ensure_capacity(&mut self, capacity: usize) {
        if let SmallEntitySetInner::Set(set) = &mut self.0 {
            set.ensure_capacity(capacity);
        }
    }

    /// Like `ensure_capacity` but returns an error on allocation failure.
    #[inline]
    pub fn try_ensure_capacity(&mut self, capacity: usize) -> Result<(), OutOfMemory> {
        match &mut self.0 {
            SmallEntitySetInner::Small(_) => Ok(()),
            SmallEntitySetInner::Set(set) => set.try_ensure_capacity(capacity),
        }
    }

    /// The inline elements that are present, in sorted order.
    #[inline]
    fn small_elems(elems: &[K; N]) -> &[K] {
        let len = elems
            .iter()
            .position(|k| k.is_reserved_value())
            .unwrap_or(N);
        &elems[..len]
    }

    /// Is `k` in this set?
    #[inline]
    pub fn contains(&self, k: K) -> bool {
        match &self.0 {
            SmallEntitySetInner::Small(elems) => Self::small_elems(elems).contains(&k),
            SmallEntitySetInner::Set(set) => set.contains(k),
        }
    }

    /// Get the number of elements in this set.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.0 {
            SmallEntitySetInner::Small(elems) => Self::small_elems(elems).len(),
            SmallEntitySetInner::Set(set) => set.len(),
        }
    }

    /// Is this set completely empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            SmallEntitySetInner::Small(elems) => {
                elems.first().is_none_or(|k| k.is_reserved_value())
            }
            SmallEntitySetInner::Set(set) => set.is_empty(),
        }
    }

    /// Remove all entries from this set.
    #[inline]
    pub fn clear(&mut self) {
        match &mut self.0 {
            SmallEntitySetInner::Small(elems) => elems.fill(K::reserved_value()),
            SmallEntitySetInner::Set(set) => set.clear(),
        }
    }

    /// Iterate over all the keys up to the maximum in this set.
    ///
    /// This will yield intermediate keys on the way up to the max key, even if
    /// they are not contained within the set.
    #[inline]
    pub fn keys(&self) -> Keys<K> {
        match &self.0 {
            SmallEntitySetInner::Small(elems) => {
                Keys::with_len(Self::small_elems(elems).last().map_or(0, |k| k.index() + 1))
            }
            SmallEntitySetInner::Set(set) => set.keys(),
        }
    }

    /// Iterate over the elements of this set, in ascending index order.
    #[inline]
    pub fn iter(&self) -> SmallSetIter<'_, K> {
        SmallSetIter(match &self.0 {
            SmallEntitySetInner::Small(elems) => {
                SmallSetIterInner::Small(Self::small_elems(elems).iter())
            }
            SmallEntitySetInner::Set(set) => SmallSetIterInner::Set(set.iter()),
        })
    }

    /// Insert `k`.
    ///
    /// Returns `true` if `k` was not present in the set, i.e. this is a
    /// newly-added element. Returns `false` otherwise.
    ///
    /// `k` must not be `K`'s reserved value.
    #[inline]
    pub fn insert(&mut self, k: K) -> bool {
        debug_assert!(
            !k.is_reserved_value(),
            "cannot insert the reserved value into a `SmallEntitySet`"
        );
        match &mut self.0 {
            SmallEntitySetInner::Small(elems) => {
                let len = Self::small_elems(elems).len();
                let pos = elems[..len]
                    .iter()
                    .position(|e| e.index() >= k.index())
                    .unwrap_or(len);
                if pos < len && elems[pos] == k {
                    return false;
                }
                if len < N {
                    elems.copy_within(pos..len, pos + 1);
                    elems[pos] = k;
                    return true;
                }
                let mut set = EntitySet::new();
                set.extend(elems.iter().copied());
                set.insert(k);
                self.0 = SmallEntitySetInner::Set(set);
                true
            }
            SmallEntitySetInner::Set(set) => set.insert(k),
        }
    }

    /// Remove `k` from this set.
    ///
    /// Returns whether `k` was previously in this set or not.
    #[inline]
    pub fn remove(&mut self, k: K) -> bool {
        match &mut self.0 {
            SmallEntitySetInner::Small(elems) => {
                let len = Self::small_elems(elems).len();
                let Some(pos) = elems[..len].iter().position(|e| *e == k) else {
                    return false;
                };
                elems.copy_within(pos + 1..len, pos);
                elems[len - 1] = K::reserved_value();
                true
            }
            SmallEntitySetInner::Set(set) => set.remove(k),
        }
    }

    /// Removes and returns the highest-index entity from the set if it exists.
    #[inline]
    pub fn pop(&mut self) -> Option<K> {
        match &mut self.0 {
            SmallEntitySetInner::Small(elems) => {
                let len = Self::small_elems(elems).len();
                let last = elems[..len].last_mut()?;
                Some(core::mem::replace(last, K::reserved_value()))
            }
            SmallEntitySetInner::Set(set) => set.pop(),
        }
    }
}

/// An iterator over the elements in a `SmallEntitySet`.
pub struct SmallSetIter<'a, K>(SmallSetIterInner<'a, K>);

enum SmallSetIterInner<'a, K> {
    Small(core::slice::Iter<'a, K>),
    Set(SetIter<'a, K>),
}

impl<K> Iterator for SmallSetIter<'_, K>
where
    K: EntityRef,
{
    type Item = K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            SmallSetIterInner::Small(iter) => iter.next().copied(),
            SmallSetIterInner::Set(iter) => iter.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, string::String, vec, vec::Vec};
    use mutatis::{Mutate, check::Check, mutators as m};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct E(u32);
    crate::entity_impl!(E);

    type Small = SmallEntitySet<E, 4>;

    fn elems(s: &Small) -> Vec<E> {
        s.iter().collect()
    }

    fn is_spilled(s: &Small) -> bool {
        matches!(s.0, SmallEntitySetInner::Set(_))
    }

    #[test]
    fn empty() {
        let s = Small::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert!(!s.contains(E(0)));
        assert_eq!(elems(&s), []);
        assert_eq!(s.keys().count(), 0);
    }

    #[test]
    fn insert_sorted() {
        let mut s = Small::new();
        assert!(s.insert(E(3)));
        assert!(s.insert(E(1)));
        assert!(s.insert(E(2)));
        assert!(!s.insert(E(1)));
        assert_eq!(elems(&s), [E(1), E(2), E(3)]);
        assert_eq!(s.len(), 3);
        assert!(s.contains(E(2)));
        assert!(!s.contains(E(0)));
        assert!(!is_spilled(&s));
    }

    #[test]
    fn spill() {
        let mut s = Small::new();
        s.extend([E(9), E(7), E(5), E(3)]);
        assert!(!is_spilled(&s));
        assert!(!s.insert(E(5)));
        assert!(!is_spilled(&s));
        assert!(s.insert(E(1)));
        assert!(is_spilled(&s));
        assert_eq!(elems(&s), [E(1), E(3), E(5), E(7), E(9)]);
        assert_eq!(s.len(), 5);
        assert!(s.contains(E(9)));
    }

    #[test]
    fn remove() {
        let mut s: Small = [E(1), E(2), E(3)].into_iter().collect();
        assert!(s.remove(E(2)));
        assert!(!s.remove(E(2)));
        assert_eq!(elems(&s), [E(1), E(3)]);
        assert!(s.remove(E(3)));
        assert!(s.remove(E(1)));
        assert!(s.is_empty());
    }

    #[test]
    fn pop() {
        let mut s: Small = [E(4), E(0), E(2)].into_iter().collect();
        assert_eq!(s.pop(), Some(E(4)));
        assert_eq!(s.pop(), Some(E(2)));
        assert_eq!(s.pop(), Some(E(0)));
        assert_eq!(s.pop(), None);
    }

    #[test]
    fn clear() {
        let mut s: Small = (0..6).map(E).collect();
        s.clear();
        assert!(s.is_empty());
        let mut s: Small = (0..2).map(E).collect();
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn keys() {
        let s: Small = [E(2)].into_iter().collect();
        assert_eq!(s.keys().collect::<Vec<_>>(), [E(0), E(1), E(2)]);
    }

    #[test]
    fn capacity() {
        let mut s = Small::with_capacity(100);
        s.ensure_capacity(100);
        s.try_ensure_capacity(100).unwrap();
        assert!(!is_spilled(&s));
        let mut s: Small = (0..8).map(E).collect();
        s.ensure_capacity(100);
        s.try_ensure_capacity(100).unwrap();
        assert_eq!(s.len(), 8);
    }

    #[test]
    fn zero_inline_capacity() {
        let mut s = SmallEntitySet::<E, 0>::new();
        assert!(s.is_empty());
        assert!(s.insert(E(1)));
        assert_eq!(s.iter().collect::<Vec<_>>(), [E(1)]);
    }

    #[test]
    fn eq_and_hash_across_representations() {
        let inline: Small = [E(1), E(2)].into_iter().collect();
        let mut spilled: Small = (0..6).map(E).collect();
        for i in [0, 3, 4, 5] {
            spilled.remove(E(i));
        }
        assert!(is_spilled(&spilled));
        assert_eq!(inline, spilled);
        assert_eq!(hash(&inline), hash(&spilled));
    }

    fn hash(s: &Small) -> Vec<u64> {
        #[derive(Default)]
        struct Recorder(Vec<u64>);
        impl Hasher for Recorder {
            fn finish(&self) -> u64 {
                0
            }
            fn write(&mut self, bytes: &[u8]) {
                self.0.extend(bytes.iter().map(|b| u64::from(*b)));
            }
        }
        let mut h = Recorder::default();
        s.hash(&mut h);
        h.0
    }

    #[test]
    fn fmt_debug() {
        let s: Small = [E(4), E(2)].into_iter().collect();
        assert_eq!(format!("{s:?}"), "{E(2), E(4)}");
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn insert_reserved_value() {
        Small::new().insert(E::reserved_value());
    }

    /// Differentially test `SmallEntitySet` against `EntitySet`.
    #[test]
    fn matches_entity_set() -> mutatis::check::CheckResult<Vec<Op>> {
        let corpus = [
            vec![],
            vec![Op::Insert(1), Op::Insert(0), Op::Remove(1), Op::Pop],
            (0..10).map(Op::Insert).collect(),
        ];
        Check::new()
            .iters(10_000)
            .run_with(m::default::<Vec<Op>>(), corpus, check_ops)
    }

    #[derive(Clone, Copy, Debug, Mutate)]
    enum Op {
        Insert(u8),
        Remove(u8),
        Contains(u8),
        Pop,
        Clear,
    }

    fn check_ops(ops: &Vec<Op>) -> Result<(), String> {
        let mut small = Small::new();
        let mut set = EntitySet::<E>::new();
        for op in ops {
            let (a, b): (String, String) = match *op {
                Op::Insert(i) => (
                    format!("{:?}", small.insert(E(i.into()))),
                    format!("{:?}", set.insert(E(i.into()))),
                ),
                Op::Remove(i) => (
                    format!("{:?}", small.remove(E(i.into()))),
                    format!("{:?}", set.remove(E(i.into()))),
                ),
                Op::Contains(i) => (
                    format!("{:?}", small.contains(E(i.into()))),
                    format!("{:?}", set.contains(E(i.into()))),
                ),
                Op::Pop => (format!("{:?}", small.pop()), format!("{:?}", set.pop())),
                Op::Clear => {
                    small.clear();
                    set.clear();
                    (String::new(), String::new())
                }
            };
            if a != b {
                return Err(format!("{op:?}: {a:?} != {b:?}"));
            }
            let small_elems = small.iter().collect::<Vec<_>>();
            let set_elems = set.iter().collect::<Vec<_>>();
            if small_elems != set_elems {
                return Err(format!("after {op:?}: {small_elems:?} != {set_elems:?}"));
            }
            if small.len() != set.len() || small.is_empty() != set.is_empty() {
                return Err(format!("after {op:?}: length mismatch"));
            }
            if small.keys().len() != set.keys().len() {
                return Err(format!("after {op:?}: keys mismatch"));
            }
        }
        Ok(())
    }
}
