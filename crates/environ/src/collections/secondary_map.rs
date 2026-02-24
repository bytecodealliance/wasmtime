use crate::{
    EntityRef,
    collections::{TryClone, Vec},
    error::OutOfMemory,
};
use core::{cmp::Ordering, fmt, marker::PhantomData, mem, ops::Index};

/// Like [`cranelift_entity::SecondaryMap`] but all allocation is fallible.
pub struct SecondaryMap<K, V> {
    elems: Vec<V>,
    default_value: V,
    phantom: PhantomData<fn(K) -> V>,
}

impl<K, V> fmt::Debug for SecondaryMap<K, V>
where
    K: EntityRef + fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entries<'a, K, V>(&'a SecondaryMap<K, V>);

        impl<'a, K, V> fmt::Debug for Entries<'a, K, V>
        where
            K: EntityRef + fmt::Debug,
            V: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map().entries(self.0.iter()).finish()
            }
        }

        f.debug_struct("SecondaryMap")
            .field("entries", &Entries(self))
            .finish()
    }
}

impl<K, V> SecondaryMap<K, V> {
    /// Same as [`cranelift_entity::SecondaryMap::new`].
    pub fn new() -> Self
    where
        V: Default,
    {
        Self {
            elems: Vec::new(),
            default_value: V::default(),
            phantom: PhantomData,
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::try_with_capacity`].
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory>
    where
        V: Default,
    {
        Ok(Self {
            elems: Vec::with_capacity(capacity)?,
            default_value: V::default(),
            phantom: PhantomData,
        })
    }

    /// Same as [`cranelift_entity::SecondaryMap::with_default`].
    pub fn with_default(default_value: V) -> Self {
        Self {
            elems: Vec::new(),
            default_value,
            phantom: PhantomData,
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::capacity`].
    pub fn capacity(&self) -> usize {
        self.elems.capacity()
    }

    /// Same as [`cranelift_entity::SecondaryMap::get`].
    pub fn get(&self, k: K) -> Option<&V>
    where
        K: EntityRef,
    {
        self.elems.get(k.index())
    }

    /// Same as [`cranelift_entity::SecondaryMap::get_mut`].
    pub fn get_mut(&mut self, k: K) -> Option<&mut V>
    where
        K: EntityRef,
    {
        self.elems.get_mut(k.index())
    }

    /// Same as [`cranelift_entity::SecondaryMap::try_insert`].
    pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, OutOfMemory>
    where
        K: EntityRef,
        V: TryClone,
    {
        if k.index() < self.elems.len() {
            Ok(Some(mem::replace(&mut self.elems[k.index()], v)))
        } else {
            self.resize(k.index() + 1)?;
            self.elems[k.index()] = v;
            Ok(None)
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::remove`] but returns an error
    /// if `TryClone`ing the default value fails when overwriting the old entry,
    /// if any.
    pub fn remove(&mut self, k: K) -> Result<Option<V>, OutOfMemory>
    where
        K: EntityRef,
        V: TryClone,
    {
        if k.index() < self.elems.len() {
            let default = self.default_value.try_clone()?;
            Ok(Some(mem::replace(&mut self.elems[k.index()], default)))
        } else {
            Ok(None)
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    /// Same as [`cranelift_entity::SecondaryMap::clear`].
    pub fn clear(&mut self) {
        self.elems.clear();
    }

    /// Same as [`cranelift_entity::SecondaryMap::iter`].
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.elems.iter().enumerate(),
            phantom: PhantomData,
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::iter_mut`].
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: self.elems.iter_mut().enumerate(),
            phantom: PhantomData,
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::keys`].
    pub fn keys(&self) -> Keys<K> {
        Keys {
            inner: 0..self.elems.len(),
            phantom: PhantomData,
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::values`].
    pub fn values(&self) -> core::slice::Iter<'_, V> {
        self.elems.iter()
    }

    /// Same as [`cranelift_entity::SecondaryMap::values_mut`].
    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, V> {
        self.elems.iter_mut()
    }

    /// Resize the map to have `n` entries by adding default entries as needed.
    pub fn resize(&mut self, n: usize) -> Result<(), OutOfMemory>
    where
        V: TryClone,
    {
        match self.elems.len().cmp(&n) {
            Ordering::Less => self.elems.resize(n, self.default_value.try_clone()?)?,
            Ordering::Equal => {}
            Ordering::Greater => self.elems.truncate(n),
        }
        Ok(())
    }
}

impl<K, V> Default for SecondaryMap<K, V>
where
    V: Default,
{
    fn default() -> SecondaryMap<K, V> {
        SecondaryMap::new()
    }
}

// NB: no `IndexMut` implementation because it requires allocation but the trait
// doesn't allow for fallibility.
impl<K, V> Index<K> for SecondaryMap<K, V>
where
    K: EntityRef,
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        self.get(k).unwrap_or(&self.default_value)
    }
}

impl<K, V> From<Vec<V>> for SecondaryMap<K, V>
where
    K: EntityRef,
    V: TryClone + Default,
{
    fn from(elems: Vec<V>) -> Self {
        Self {
            elems,
            default_value: V::default(),
            phantom: PhantomData,
        }
    }
}

impl<K, V> serde::ser::Serialize for SecondaryMap<K, V>
where
    K: EntityRef,
    V: PartialEq + serde::ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq as _;

        // Ignore any trailing default values.
        let mut len = self.capacity();
        while len > 0 && &self[K::new(len - 1)] == &self.default_value {
            len -= 1;
        }

        // Plus one for the default value.
        let mut seq = serializer.serialize_seq(Some(len + 1))?;

        // Always serialize the default value first.
        seq.serialize_element(&self.default_value)?;

        for elem in self.values().take(len) {
            let elem = if elem == &self.default_value {
                None
            } else {
                Some(elem)
            };
            seq.serialize_element(&elem)?;
        }

        seq.end()
    }
}

impl<'de, K, V> serde::de::Deserialize<'de> for SecondaryMap<K, V>
where
    K: EntityRef,
    V: TryClone + serde::de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<K, V>(core::marker::PhantomData<fn() -> SecondaryMap<K, V>>)
        where
            K: EntityRef,
            V: TryClone;

        impl<'de, K, V> serde::de::Visitor<'de> for Visitor<K, V>
        where
            K: EntityRef,
            V: TryClone + serde::de::Deserialize<'de>,
        {
            type Value = SecondaryMap<K, V>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct SecondaryMap")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                // Minus one to account for the default element, which is always
                // the first in the sequence.
                let size_hint = seq.size_hint().and_then(|n| n.checked_sub(1));

                let Some(default) = seq.next_element::<V>()? else {
                    return Err(serde::de::Error::custom("Default value required"));
                };

                let mut map = SecondaryMap::<K, V>::with_default(
                    default
                        .try_clone()
                        .map_err(|oom| serde::de::Error::custom(oom))?,
                );

                if let Some(n) = size_hint {
                    map.resize(n).map_err(|oom| serde::de::Error::custom(oom))?;
                }

                let mut idx = 0;
                while let Some(val) = seq.next_element::<Option<V>>()? {
                    let key = K::new(idx);
                    let val = match val {
                        None => default
                            .try_clone()
                            .map_err(|oom| serde::de::Error::custom(oom))?,
                        Some(val) => val,
                    };

                    map.insert(key, val)
                        .map_err(|oom| serde::de::Error::custom(oom))?;

                    idx += 1;
                }

                Ok(map)
            }
        }

        deserializer.deserialize_seq(Visitor(core::marker::PhantomData))
    }
}

/// A shared iterator over a `SecondaryMap<K, V>`.
pub struct Iter<'a, K, V> {
    inner: core::iter::Enumerate<core::slice::Iter<'a, V>>,
    phantom: PhantomData<fn() -> K>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: EntityRef,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let (i, v) = self.inner.next()?;
        Some((K::new(i), v))
    }
}

/// An exclusive iterator over a `SecondaryMap<K, V>`.
pub struct IterMut<'a, K, V> {
    inner: core::iter::Enumerate<core::slice::IterMut<'a, V>>,
    phantom: PhantomData<fn() -> K>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: EntityRef,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let (i, v) = self.inner.next()?;
        Some((K::new(i), v))
    }
}

/// An iterator over the keys in a `SecondaryMap<K, V>`.
pub struct Keys<K> {
    inner: core::ops::Range<usize>,
    phantom: PhantomData<fn() -> K>,
}

impl<K> Iterator for Keys<K>
where
    K: EntityRef,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        Some(K::new(self.inner.next()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use alloc::vec;
    use alloc::vec::Vec;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct K(u32);
    crate::entity_impl!(K);

    fn k(i: usize) -> K {
        K::new(i)
    }

    #[test]
    fn with_capacity() -> Result<()> {
        let map = SecondaryMap::<K, u32>::with_capacity(100)?;
        assert!(map.capacity() >= 100);
        Ok(())
    }

    #[test]
    fn with_default() -> Result<()> {
        let map = SecondaryMap::<K, u32>::with_default(42);
        assert_eq!(map[k(99)], 42);
        Ok(())
    }

    #[test]
    fn get() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(10), 99)?;
        assert_eq!(map.get(k(0)).copied(), Some(0));
        assert_eq!(map.get(k(10)).copied(), Some(99));
        assert!(map.get(k(100)).is_none());
        Ok(())
    }

    #[test]
    fn get_mut() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(10), 99)?;
        *map.get_mut(k(0)).unwrap() += 1;
        *map.get_mut(k(10)).unwrap() += 1;
        assert_eq!(map[k(0)], 1);
        assert_eq!(map[k(10)], 100);
        assert!(map.get_mut(k(100)).is_none());
        Ok(())
    }

    #[test]
    fn insert() -> Result<()> {
        let mut map = SecondaryMap::new();
        assert_eq!(map[k(3)], 0);
        map.insert(k(3), 99)?;
        assert_eq!(map[k(3)], 99);
        Ok(())
    }

    #[test]
    fn remove() -> Result<()> {
        let mut map = SecondaryMap::new();

        let old = map.remove(k(3))?;
        assert!(old.is_none());

        map.insert(k(3), 99)?;

        let old = map.remove(k(3))?;
        assert_eq!(old, Some(99));
        assert_eq!(map[k(3)], 0);

        let old = map.remove(k(3))?;
        assert_eq!(old, Some(0));

        Ok(())
    }

    #[test]
    fn is_empty() -> Result<()> {
        let mut map = SecondaryMap::new();
        assert!(map.is_empty());
        map.insert(k(0), 1)?;
        assert!(!map.is_empty());
        Ok(())
    }

    #[test]
    fn clear() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(0), 1)?;
        map.clear();
        assert!(map.is_empty());
        assert_eq!(map[k(0)], 0);
        Ok(())
    }

    #[test]
    fn iter() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(0), 'a')?;
        map.insert(k(1), 'b')?;
        map.insert(k(5), 'c')?;
        assert_eq!(
            map.iter().collect::<Vec<_>>(),
            vec![
                (k(0), &'a'),
                (k(1), &'b'),
                (k(2), &char::default()),
                (k(3), &char::default()),
                (k(4), &char::default()),
                (k(5), &'c'),
            ],
        );
        Ok(())
    }

    #[test]
    fn iter_mut() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(0), 'a')?;
        map.insert(k(1), 'b')?;
        map.insert(k(5), 'c')?;
        assert_eq!(
            map.iter_mut().collect::<Vec<_>>(),
            vec![
                (k(0), &mut 'a'),
                (k(1), &mut 'b'),
                (k(2), &mut char::default()),
                (k(3), &mut char::default()),
                (k(4), &mut char::default()),
                (k(5), &mut 'c'),
            ],
        );
        Ok(())
    }

    #[test]
    fn keys() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(2), 9)?;
        assert_eq!(map.keys().collect::<Vec<_>>(), vec![k(0), k(1), k(2)]);
        Ok(())
    }

    #[test]
    fn values() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(2), 9)?;
        assert_eq!(map.values().collect::<Vec<_>>(), vec![&0, &0, &9]);
        Ok(())
    }

    #[test]
    fn values_mut() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(2), 9)?;
        assert_eq!(
            map.values_mut().collect::<Vec<_>>(),
            vec![&mut 0, &mut 0, &mut 9]
        );
        Ok(())
    }

    #[test]
    fn resize() -> Result<()> {
        let mut map = SecondaryMap::<K, u32>::new();
        assert!(map.is_empty());

        // Grow via resize.
        map.resize(2)?;
        assert!(!map.is_empty());
        assert!(map.get(k(0)).is_some());
        assert!(map.get(k(1)).is_some());
        assert!(map.get(k(2)).is_none());

        // Resize to same size.
        map.resize(2)?;
        assert!(!map.is_empty());
        assert!(map.get(k(0)).is_some());
        assert!(map.get(k(1)).is_some());
        assert!(map.get(k(2)).is_none());

        // Shrink via resize.
        map.resize(1)?;
        assert!(!map.is_empty());
        assert!(map.get(k(0)).is_some());
        assert!(map.get(k(1)).is_none());

        Ok(())
    }

    #[test]
    fn index() -> Result<()> {
        let mut map = SecondaryMap::new();
        map.insert(k(0), 55)?;
        assert_eq!(map[k(0)], 55);
        assert_eq!(map[k(999)], 0);
        Ok(())
    }

    #[test]
    fn from_vec() -> Result<()> {
        let v = crate::collections::vec![10, 20, 30]?;
        let map = SecondaryMap::from(v);
        assert_eq!(map[k(0)], 10);
        assert_eq!(map[k(1)], 20);
        assert_eq!(map[k(2)], 30);
        assert_eq!(map[k(3)], 0);
        Ok(())
    }

    #[test]
    fn serialize_deserialize() -> Result<()> {
        let mut map = SecondaryMap::<K, u32>::with_default(99);
        map.insert(k(0), 33)?;
        map.insert(k(1), 44)?;
        map.insert(k(2), 55)?;
        map.insert(k(3), 99)?;
        map.insert(k(4), 99)?;

        let bytes = postcard::to_allocvec(&map)?;
        let map2: SecondaryMap<K, u32> = postcard::from_bytes(&bytes)?;

        for i in 0..10 {
            assert_eq!(map[k(i)], map2[k(i)]);
        }

        // Trailing default entries were omitted from the serialization.
        assert_eq!(map2.keys().collect::<Vec<_>>(), vec![k(0), k(1), k(2)]);

        Ok(())
    }
}
