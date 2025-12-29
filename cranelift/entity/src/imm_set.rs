//! Immutable entity sets.

use super::EntityRef;
use core::{fmt, marker::PhantomData, mem};
use cranelift_bitset::ScalarBitSet;

/// An immutable, persistent version of an [`EntitySet`][crate::EntitySet].
#[derive(Clone)]
pub struct ImmutableEntitySet<K> {
    words: im_rc::OrdMap<u32, ScalarBitSet<usize>>,
    len: u32,
    _phantom: PhantomData<K>,
}

impl<K> Default for ImmutableEntitySet<K> {
    fn default() -> Self {
        Self {
            words: Default::default(),
            len: 0,
            _phantom: Default::default(),
        }
    }
}

impl<K: fmt::Debug + EntityRef> fmt::Debug for ImmutableEntitySet<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<K> ImmutableEntitySet<K>
where
    K: EntityRef,
{
    const BITS_PER_WORD: usize = mem::size_of::<usize>() * 8;

    #[inline]
    fn word_and_bit(key: K) -> (u32, u8) {
        let key_index = key.index();
        let bit = key_index % Self::BITS_PER_WORD;
        let word = key_index / Self::BITS_PER_WORD;
        (u32::try_from(word).unwrap(), u8::try_from(bit).unwrap())
    }

    #[inline]
    fn key_from_word_and_bit(word: u32, bit: u8) -> K {
        let word = usize::try_from(word).unwrap();
        let bit = usize::try_from(bit).unwrap();
        K::new(word * Self::BITS_PER_WORD + bit)
    }

    /// Create a set containing just the given key.
    #[inline]
    pub fn unit(key: K) -> Self {
        let (word, bit) = Self::word_and_bit(key);
        let mut bitset = ScalarBitSet::new();
        bitset.insert(bit);
        ImmutableEntitySet {
            words: im_rc::OrdMap::unit(word, bitset),
            len: 1,
            _phantom: PhantomData,
        }
    }

    /// Insert a new key into this set.
    ///
    /// Returns `true` if the set did not previously contain the key, `false`
    /// otherwise.
    #[inline]
    pub fn insert(&mut self, key: K) -> bool {
        let (word, bit) = Self::word_and_bit(key);
        let bitset = self.words.entry(word).or_default();
        let is_new = bitset.insert(bit);
        self.len += u32::from(is_new);
        is_new
    }

    /// Does this set contain the given key?
    #[inline]
    pub fn contains(&self, key: K) -> bool {
        let (word, bit) = Self::word_and_bit(key);
        self.words.get(&word).is_some_and(|bits| bits.contains(bit))
    }

    /// Get the number of elements in this set.
    #[inline]
    pub fn len(&self) -> usize {
        usize::try_from(self.len).unwrap()
    }

    /// Iterate over the keys in this set, in order.
    #[inline]
    pub fn iter(&self) -> ImmutableEntitySetIter<'_, K> {
        ImmutableEntitySetIter {
            words: self.words.iter(),
            word_and_bits: None,
            _phantom: PhantomData,
        }
    }
}

/// An iterator over the entries in an [`ImmutableEntitySet`].
pub struct ImmutableEntitySetIter<'a, K> {
    words: im_rc::ordmap::Iter<'a, u32, ScalarBitSet<usize>>,
    word_and_bits: Option<(u32, cranelift_bitset::scalar::Iter<usize>)>,
    _phantom: PhantomData<K>,
}

impl<K> Iterator for ImmutableEntitySetIter<'_, K>
where
    K: EntityRef,
{
    type Item = K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (word, bits) = {
                if self.word_and_bits.is_none() {
                    let (&word, bits) = self.words.next()?;
                    self.word_and_bits = Some((word, bits.iter()));
                }
                // Safety: we replaced `None` with `Some` just above.
                unsafe { self.word_and_bits.as_mut().unwrap_unchecked() }
            };

            let Some(bit) = bits.next() else {
                self.word_and_bits = None;
                continue;
            };

            return Some(ImmutableEntitySet::key_from_word_and_bit(*word, bit));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Key(u32);
    crate::entity_impl!(Key);

    #[test]
    fn smoke_test() {
        let mut set = ImmutableEntitySet::default();

        for i in 0..100 {
            let is_new = set.insert(Key::new(i));
            assert!(is_new);
        }
        for i in 0..100 {
            let is_new = set.insert(Key::new(i));
            assert!(!is_new);
        }

        for i in 900..1000 {
            let is_new = set.insert(Key::new(i));
            assert!(is_new);
        }
        for i in 900..1000 {
            let is_new = set.insert(Key::new(i));
            assert!(!is_new);
        }

        for i in u32::MAX - 100..u32::MAX {
            let i = usize::try_from(i).unwrap();
            let is_new = set.insert(Key::new(i));
            assert!(is_new);
        }
        for i in u32::MAX - 100..u32::MAX {
            let i = usize::try_from(i).unwrap();
            let is_new = set.insert(Key::new(i));
            assert!(!is_new);
        }

        for i in 0..100 {
            assert!(set.contains(Key::new(i)));
        }
        for i in 100..200 {
            assert!(!set.contains(Key::new(i)));
        }

        for i in 800..900 {
            assert!(!set.contains(Key::new(i)));
        }
        for i in 900..1000 {
            assert!(set.contains(Key::new(i)));
        }
        for i in 1000..1100 {
            assert!(!set.contains(Key::new(i)));
        }

        for i in u32::MAX - 200..u32::MAX - 100 {
            let i = usize::try_from(i).unwrap();
            assert!(!set.contains(Key::new(i)));
        }
        for i in u32::MAX - 100..u32::MAX {
            let i = usize::try_from(i).unwrap();
            assert!(set.contains(Key::new(i)));
        }

        assert_eq!(set.len(), 300);
        assert_eq!(set.iter().count(), 300);
        for k in set.iter() {
            assert!(set.contains(k));
        }
    }

    #[test]
    fn unit() {
        let set = ImmutableEntitySet::unit(Key::new(42));

        assert!(set.contains(Key::new(42)));

        assert!(!set.contains(Key::new(0)));
        assert!(!set.contains(Key::new(41)));
        assert!(!set.contains(Key::new(43)));

        assert_eq!(set.iter().collect::<Vec<_>>(), [Key::new(42)]);
    }

    #[test]
    fn iter() {
        let mut set = ImmutableEntitySet::default();
        set.insert(Key::new(0));
        set.insert(Key::new(1));
        set.insert(Key::new(2));
        set.insert(Key::new(31));
        set.insert(Key::new(32));
        set.insert(Key::new(33));
        set.insert(Key::new(63));
        set.insert(Key::new(64));
        set.insert(Key::new(65));
        set.insert(Key::new(usize::try_from(u32::MAX - 1).unwrap()));

        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [
                Key::new(0),
                Key::new(1),
                Key::new(2),
                Key::new(31),
                Key::new(32),
                Key::new(33),
                Key::new(63),
                Key::new(64),
                Key::new(65),
                Key::new(usize::try_from(u32::MAX - 1).unwrap()),
            ]
        );
    }
}
