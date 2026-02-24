use crate::{collections::Vec, error::OutOfMemory};
use core::{fmt, ops::Index};
use cranelift_entity::{EntityRef, SecondaryMap as Inner};

/// Like [`cranelift_entity::SecondaryMap`] but all allocation is fallible.
pub struct SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    inner: Inner<K, V>,
}

impl<K, V> fmt::Debug for SecondaryMap<K, V>
where
    K: EntityRef + fmt::Debug,
    V: fmt::Debug + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<K, V> SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    /// Same as [`cranelift_entity::SecondaryMap::new`].
    pub fn new() -> Self
    where
        V: Default,
    {
        Self {
            inner: Inner::new(),
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::try_with_capacity`].
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory>
    where
        V: Default,
    {
        Ok(Self {
            inner: Inner::try_with_capacity(capacity)?,
        })
    }

    /// Same as [`cranelift_entity::SecondaryMap::with_default`].
    pub fn with_default(default: V) -> Self {
        Self {
            inner: Inner::with_default(default),
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::capacity`].
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Same as [`cranelift_entity::SecondaryMap::get`].
    pub fn get(&self, k: K) -> Option<&V> {
        self.inner.get(k)
    }

    /// Same as [`cranelift_entity::SecondaryMap::get_mut`].
    pub fn get_mut(&mut self, k: K) -> Option<&mut V> {
        self.inner.get_mut(k)
    }

    /// Same as [`cranelift_entity::SecondaryMap::try_insert`].
    pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, OutOfMemory> {
        self.inner.try_insert(k, v)
    }

    /// Same as [`cranelift_entity::SecondaryMap::remove`].
    pub fn remove(&mut self, k: K) -> Option<V> {
        self.inner.remove(k)
    }

    /// Same as [`cranelift_entity::SecondaryMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`cranelift_entity::SecondaryMap::clear`].
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Same as [`cranelift_entity::SecondaryMap::iter`].
    pub fn iter(&self) -> cranelift_entity::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Same as [`cranelift_entity::SecondaryMap::iter_mut`].
    pub fn iter_mut(&mut self) -> cranelift_entity::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    /// Same as [`cranelift_entity::SecondaryMap::keys`].
    pub fn keys(&self) -> cranelift_entity::Keys<K> {
        self.inner.keys()
    }

    /// Same as [`cranelift_entity::SecondaryMap::values`].
    pub fn values(&self) -> core::slice::Iter<'_, V> {
        self.inner.values()
    }

    /// Same as [`cranelift_entity::SecondaryMap::values_mut`].
    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, V> {
        self.inner.values_mut()
    }

    /// Resize the map to have `n` entries by adding default entries as needed.
    pub fn resize(&mut self, n: usize) -> Result<(), OutOfMemory> {
        self.inner.try_resize(n)
    }
}

impl<K, V> Default for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone + Default,
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
    V: Clone,
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        &self.inner[k]
    }
}

impl<K, V> From<Vec<V>> for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone + Default,
{
    fn from(values: Vec<V>) -> Self {
        let values: alloc::vec::Vec<V> = values.into();
        let inner = Inner::from(values);
        Self { inner }
    }
}

impl<K, V> serde::ser::Serialize for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone + PartialEq + serde::ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq as _;

        // Ignore any trailing default values.
        let mut len = self.inner.as_values_slice().len();
        while len > 0 && &self[K::new(len - 1)] == self.inner.default_value() {
            len -= 1;
        }

        // Plus one for the default value.
        let mut seq = serializer.serialize_seq(Some(len + 1))?;

        // Always serialize the default value first.
        seq.serialize_element(self.inner.default_value())?;

        for elem in self.values().take(len) {
            let elem = if elem == self.inner.default_value() {
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
    V: Clone + serde::de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<K, V>(core::marker::PhantomData<fn() -> SecondaryMap<K, V>>)
        where
            K: EntityRef,
            V: Clone;

        impl<'de, K, V> serde::de::Visitor<'de> for Visitor<K, V>
        where
            K: EntityRef,
            V: Clone + serde::de::Deserialize<'de>,
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

                let mut map = SecondaryMap::<K, V>::with_default(default.clone());

                if let Some(n) = size_hint {
                    map.resize(n).map_err(|oom| serde::de::Error::custom(oom))?;
                }

                let mut idx = 0;
                while let Some(val) = seq.next_element::<Option<V>>()? {
                    let key = K::new(idx);
                    let val = match val {
                        None => default.clone(),
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
