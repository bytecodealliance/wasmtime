//! Compound bit sets.

use crate::scalar::{self, ScalarBitSet};
use alloc::{vec, vec::Vec};
use core::mem;

/// A large bit set backed by dynamically-sized storage.
///
/// # Example
///
/// ```
/// use cranelift_bitset::CompoundBitSet;
///
/// // Create a new bitset.
/// let mut bitset = CompoundBitSet::new();
///
/// // Bitsets are initially empty.
/// assert!(bitset.is_empty());
/// assert_eq!(bitset.len(), 0);
///
/// // Insert into the bitset.
/// bitset.insert(444);
/// bitset.insert(555);
/// bitset.insert(666);
///
/// // Now the bitset is not empty.
/// assert_eq!(bitset.len(), 3);
/// assert!(!bitset.is_empty());
/// assert!(bitset.contains(444));
/// assert!(bitset.contains(555));
/// assert!(bitset.contains(666));
///
/// // Remove an element from the bitset.
/// let was_present = bitset.remove(666);
/// assert!(was_present);
/// assert!(!bitset.contains(666));
/// assert_eq!(bitset.len(), 2);
///
/// // Can iterate over the elements in the set.
/// let elems: Vec<_> = bitset.iter().collect();
/// assert_eq!(elems, [444, 555]);
/// ```
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct CompoundBitSet {
    elems: Vec<ScalarBitSet<usize>>,
    len: usize,
    max: Option<usize>,
}

impl core::fmt::Debug for CompoundBitSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CompoundBitSet ")?;
        f.debug_set().entries(self.iter()).finish()
    }
}

impl Default for CompoundBitSet {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

const BITS_PER_WORD: usize = mem::size_of::<usize>() * 8;

impl CompoundBitSet {
    /// Construct a new, empty bit set.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let bitset = CompoundBitSet::new();
    ///
    /// assert!(bitset.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        CompoundBitSet {
            elems: vec![],
            len: 0,
            max: None,
        }
    }

    /// Construct a new, empty bit set with space reserved to store any element
    /// `x` such that `x < capacity`.
    ///
    /// The actual capacity reserved may be greater than that requested.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let bitset = CompoundBitSet::with_capacity(4096);
    ///
    /// assert!(bitset.is_empty());
    /// assert!(bitset.capacity() >= 4096);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        // Divide `capacity` by `BITS_PER_WORD`, rounding up rather than down.
        let elems_cap = (capacity + BITS_PER_WORD - 1) / BITS_PER_WORD;
        CompoundBitSet {
            elems: Vec::with_capacity(elems_cap),
            len: 0,
            max: None,
        }
    }

    /// Get the number of elements in this bitset.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// assert_eq!(bitset.len(), 0);
    ///
    /// bitset.insert(24);
    /// bitset.insert(130);
    /// bitset.insert(3600);
    ///
    /// assert_eq!(bitset.len(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get `n + 1` where `n` is the largest value that can be stored inside
    /// this set without growing the backing storage.
    ///
    /// That is, this set can store any value `x` such that `x <
    /// bitset.capacity()` without growing the backing storage.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// // New bitsets have zero capacity -- they allocate lazily.
    /// assert_eq!(bitset.capacity(), 0);
    ///
    /// // Insert into the bitset, growing its capacity.
    /// bitset.insert(999);
    ///
    /// // The bitset must now have capacity for at least `999` elements,
    /// // perhaps more.
    /// assert!(bitset.capacity() >= 999);
    ///```
    pub fn capacity(&self) -> usize {
        self.elems.capacity() * BITS_PER_WORD
    }

    /// Is this bitset empty?
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// assert!(bitset.is_empty());
    ///
    /// bitset.insert(1234);
    ///
    /// assert!(!bitset.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Convert an element `i` into the `word` that can be used to index into
    /// `self.elems` and the `bit` that can be tested in the
    /// `ScalarBitSet<usize>` at `self.elems[word]`.
    #[inline]
    fn word_and_bit(i: usize) -> (usize, u8) {
        let word = i / BITS_PER_WORD;
        let bit = i % BITS_PER_WORD;
        let bit = u8::try_from(bit).unwrap();
        (word, bit)
    }

    /// The opposite of `word_and_bit`: convert the pair of an index into
    /// `self.elems` and associated bit index into a set element.
    #[inline]
    fn elem(word: usize, bit: u8) -> usize {
        let bit = usize::from(bit);
        debug_assert!(bit < BITS_PER_WORD);
        word * BITS_PER_WORD + bit
    }

    /// Is `i` contained in this bitset?
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// assert!(!bitset.contains(666));
    ///
    /// bitset.insert(666);
    ///
    /// assert!(bitset.contains(666));
    /// ```
    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        let (word, bit) = Self::word_and_bit(i);
        if word < self.elems.len() {
            self.elems[word].contains(bit)
        } else {
            false
        }
    }

    /// Reserve space in this bitset for the values `0..n`, growing the backing
    /// storage if necessary.
    ///
    /// After calling `bitset.reserve(n)`, inserting any element `i` where `i <
    /// n` is guaranteed to succeed without growing the bitset's backing
    /// storage.
    ///
    /// # Example
    ///
    /// ```
    /// # use cranelift_bitset::CompoundBitSet;
    /// # let mut bitset = CompoundBitSet::new();
    /// // We are going to do a series of inserts where `1000` will be the
    /// // maximum value inserted. Make sure that our bitset has capacity for
    /// // these elements once up front, to avoid growing the backing storage
    /// // multiple times incrementally.
    /// bitset.reserve(1001);
    ///
    /// for i in 0..=1000 {
    ///     if i % 2 == 0 {
    ///         // Inserting this value should not require growing the backing
    ///         // storage.
    ///         assert!(bitset.capacity() > i);
    ///         bitset.insert(i);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        let (word, _bit) = Self::word_and_bit(n);
        if word >= self.elems.len() {
            self.elems.resize_with(word + 1, ScalarBitSet::new);
        }
    }

    /// Insert `i` into this bitset.
    ///
    /// Returns whether the value was newly inserted. That is:
    ///
    /// * If the set did not previously contain `i` then `true` is returned.
    ///
    /// * If the set already contained `i` then `false` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// // When an element is inserted that was not already present in the set,
    /// // then `true` is returned.
    /// let is_new = bitset.insert(1234);
    /// assert!(is_new);
    ///
    /// // The element is now present in the set.
    /// assert!(bitset.contains(1234));
    ///
    /// // And when the element is already in the set, `false` is returned from
    /// // `insert`.
    /// let is_new = bitset.insert(1234);
    /// assert!(!is_new);
    /// ```
    #[inline]
    pub fn insert(&mut self, i: usize) -> bool {
        self.reserve(i + 1);
        let (word, bit) = Self::word_and_bit(i);
        let is_new = self.elems[word].insert(bit);
        self.len += is_new as usize;
        self.max = self.max.map(|max| core::cmp::max(max, i)).or(Some(i));
        is_new
    }

    /// Remove `i` from this bitset.
    ///
    /// Returns whether `i` was previously in this set or not.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// // Removing an element that was not present in the set returns `false`.
    /// let was_present = bitset.remove(456);
    /// assert!(!was_present);
    ///
    /// // And when the element was in the set, `true` is returned.
    /// bitset.insert(456);
    /// let was_present = bitset.remove(456);
    /// assert!(was_present);
    /// ```
    #[inline]
    pub fn remove(&mut self, i: usize) -> bool {
        let (word, bit) = Self::word_and_bit(i);
        if word < self.elems.len() {
            let sub = &mut self.elems[word];
            let was_present = sub.remove(bit);
            self.len -= was_present as usize;
            if was_present && self.max == Some(i) {
                self.update_max(word);
            }
            was_present
        } else {
            false
        }
    }

    /// Update the `self.max` field, based on the old word index of `self.max`.
    fn update_max(&mut self, word_of_old_max: usize) {
        self.max = self.elems[0..word_of_old_max + 1]
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(word, sub)| {
                let bit = sub.max()?;
                Some(Self::elem(word, bit))
            })
            .next();
    }

    /// Get the largest value in this set, or `None` if this set is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// // Returns `None` if the bitset is empty.
    /// assert!(bitset.max().is_none());
    ///
    /// bitset.insert(123);
    /// bitset.insert(987);
    /// bitset.insert(999);
    ///
    /// // Otherwise, it returns the largest value in the set.
    /// assert_eq!(bitset.max(), Some(999));
    /// ```
    #[inline]
    pub fn max(&self) -> Option<usize> {
        self.max
    }

    /// Removes and returns the largest value in this set.
    ///
    /// Returns `None` if this set is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// bitset.insert(111);
    /// bitset.insert(222);
    /// bitset.insert(333);
    /// bitset.insert(444);
    /// bitset.insert(555);
    ///
    /// assert_eq!(bitset.pop(), Some(555));
    /// assert_eq!(bitset.pop(), Some(444));
    /// assert_eq!(bitset.pop(), Some(333));
    /// assert_eq!(bitset.pop(), Some(222));
    /// assert_eq!(bitset.pop(), Some(111));
    /// assert_eq!(bitset.pop(), None);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<usize> {
        let max = self.max?;
        self.remove(max);
        Some(max)
    }

    /// Remove all elements from this bitset.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// bitset.insert(100);
    /// bitset.insert(200);
    /// bitset.insert(300);
    ///
    /// bitset.clear();
    ///
    /// assert!(bitset.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.elems.clear();
        self.len = 0;
        self.max = None;
    }

    /// Iterate over the elements in this bitset.
    ///
    /// The elements are always yielded in sorted order.
    ///
    /// # Example
    ///
    /// ```
    /// use cranelift_bitset::CompoundBitSet;
    ///
    /// let mut bitset = CompoundBitSet::new();
    ///
    /// bitset.insert(0);
    /// bitset.insert(4096);
    /// bitset.insert(123);
    /// bitset.insert(456);
    /// bitset.insert(789);
    ///
    /// assert_eq!(
    ///     bitset.iter().collect::<Vec<_>>(),
    ///     [0, 123, 456, 789, 4096],
    /// );
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            bitset: self,
            word: 0,
            sub: None,
        }
    }
}

impl<'a> IntoIterator for &'a CompoundBitSet {
    type Item = usize;

    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the elements in a [`CompoundBitSet`].
pub struct Iter<'a> {
    bitset: &'a CompoundBitSet,
    word: usize,
    sub: Option<scalar::Iter<usize>>,
}

impl Iterator for Iter<'_> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        loop {
            if let Some(sub) = &mut self.sub {
                if let Some(bit) = sub.next() {
                    return Some(CompoundBitSet::elem(self.word, bit));
                } else {
                    self.word += 1;
                }
            }

            self.sub = Some(self.bitset.elems.get(self.word)?.iter());
        }
    }
}
