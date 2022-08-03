//! A hashmap with "external hashing": nodes are hashed or compared for
//! equality only with some external context provided on lookup/insert.
//! This allows very memory-efficient data structures where
//! node-internal data references some other storage (e.g., offsets into
//! an array or pool of shared data).

use hashbrown::raw::{Bucket, RawTable};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Trait that allows for equality comparison given some external
/// context. Implemented by the *context*, for somewhat complex
/// lifetime reasons (lack of GATs to allow `for<'ctx> Ctx<'ctx>`-like
/// associated types in traits on the value type).
pub trait CtxEq<V1: ?Sized, V2: ?Sized> {
    fn ctx_eq(&self, a: &V1, b: &V2) -> bool;
}

/// Trait that allows for hashing given some external context.
pub trait CtxHash<Value: ?Sized>: CtxEq<Value, Value> {
    fn ctx_hash<H>(&self, value: &Value, state: &mut H)
    where
        H: Hasher;
}

// A null-comparator context type for underlying value types that
// already have `Eq` and `Hash`.
#[derive(Default)]
pub struct NullCtx<V: Eq + Hash> {
    _phantom: PhantomData<V>,
}

impl<V: Eq + Hash> CtxEq<V, V> for NullCtx<V> {
    fn ctx_eq(&self, a: &V, b: &V) -> bool {
        a.eq(b)
    }
}
impl<V: Eq + Hash> CtxHash<V> for NullCtx<V> {
    fn ctx_hash<H>(&self, value: &V, state: &mut H)
    where
        H: Hasher,
    {
        value.hash(state);
    }
}

struct BucketData<K, V> {
    k: K,
    v: V,
}

/// A HashMap that takes external context for all operations.
pub struct CtxHashMap<K, V> {
    raw: RawTable<BucketData<K, V>>,
}

impl<K, V> CtxHashMap<K, V> {
    /// Create an empty hashmap.
    pub fn new() -> Self {
        Self {
            raw: RawTable::new(),
        }
    }

    /// Create an empty hashmap with pre-allocated space for the given
    /// capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            raw: RawTable::with_capacity(capacity),
        }
    }
}

fn hash<K, Ctx>(k: &K, ctx: &Ctx) -> u64
where
    Ctx: CtxHash<K>,
{
    let mut hasher = fxhash::FxHasher::default();
    ctx.ctx_hash(k, &mut hasher);
    hasher.finish()
}

impl<K, V> CtxHashMap<K, V> {
    /// Insert a new key-value pair, returning the old value associated
    /// with this key (if any).
    pub fn insert<Ctx: CtxEq<K, K> + CtxHash<K>>(&mut self, k: K, v: V, ctx: &Ctx) -> Option<V> {
        let h = hash(&k, ctx);
        match self.raw.find(h, |bucket| ctx.ctx_eq(&bucket.k, &k)) {
            Some(bucket) => {
                let data = unsafe { bucket.as_mut() };
                Some(std::mem::replace(&mut data.v, v))
            }
            None => {
                let data = BucketData { k, v };
                self.raw
                    .insert_entry(h, data, |bucket| hash(&bucket.k, ctx));
                None
            }
        }
    }

    /// Look up a key, returning a borrow of the value if present.
    pub fn get<'a, Q, Ctx: CtxEq<K, Q> + CtxHash<Q> + CtxHash<K>>(
        &'a self,
        k: &Q,
        ctx: &Ctx,
    ) -> Option<&'a V> {
        let h = hash(k, ctx);
        self.raw
            .find(h, |bucket| ctx.ctx_eq(&bucket.k, k))
            .map(|bucket| {
                let data = unsafe { bucket.as_ref() };
                &data.v
            })
    }

    /// Return an Entry cursor on a given bucket for a key, allowing
    /// for fetching the current value or inserting a new one.
    pub fn entry<'a, Ctx: CtxEq<K, K> + CtxHash<K>>(
        &'a mut self,
        k: K,
        ctx: &'a Ctx,
    ) -> Entry<'a, Ctx, K, V> {
        let h = hash(&k, ctx);
        match self.raw.find(h, |bucket| ctx.ctx_eq(&bucket.k, &k)) {
            Some(bucket) => Entry::Occupied(OccupiedEntry {
                bucket,
                _phantom: PhantomData,
            }),
            None => Entry::Vacant(VacantEntry {
                raw: &mut self.raw,
                ctx,
                hash: h,
                key: k,
            }),
        }
    }
}

/// An entry in the hashmap.
pub enum Entry<'a, Ctx, K: 'a, V>
where
    Ctx: CtxHash<K>,
{
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, Ctx, K, V>),
}

/// An occupied entry.
pub struct OccupiedEntry<'a, K, V> {
    bucket: Bucket<BucketData<K, V>>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, K: 'a, V> OccupiedEntry<'a, K, V> {
    /// Get the value.
    pub fn get(&self) -> &'a V {
        let bucket = unsafe { self.bucket.as_ref() };
        &bucket.v
    }
}

/// A vacant entry.
pub struct VacantEntry<'a, Ctx, K, V>
where
    Ctx: CtxHash<K>,
{
    raw: &'a mut RawTable<BucketData<K, V>>,
    ctx: &'a Ctx,
    hash: u64,
    key: K,
}

impl<'a, Ctx, K, V> VacantEntry<'a, Ctx, K, V>
where
    Ctx: CtxHash<K>,
{
    /// Insert a value.
    pub fn insert(self, v: V) -> &'a V {
        let bucket = self
            .raw
            .insert(self.hash, BucketData { k: self.key, v }, |bucket| {
                hash(&bucket.k, self.ctx)
            });
        let data = unsafe { bucket.as_ref() };
        &data.v
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::hash::Hash;

    #[derive(Clone, Copy, Debug)]
    struct Key {
        index: u32,
    }
    struct Ctx {
        vals: &'static [&'static str],
    }
    impl CtxEq<Key, Key> for Ctx {
        fn ctx_eq(&self, a: &Key, b: &Key) -> bool {
            self.vals[a.index as usize].eq(self.vals[b.index as usize])
        }
    }
    impl CtxHash<Key> for Ctx {
        fn ctx_hash<H>(&self, value: &Key, state: &mut H)
        where
            H: Hasher,
        {
            self.vals[value.index as usize].hash(state);
        }
    }

    #[test]
    fn test_basic() {
        let ctx = Ctx {
            vals: &["a", "b", "a"],
        };

        let k0 = Key { index: 0 };
        let k1 = Key { index: 1 };
        let k2 = Key { index: 2 };

        assert!(ctx.ctx_eq(&k0, &k2));
        assert!(!ctx.ctx_eq(&k0, &k1));
        assert!(!ctx.ctx_eq(&k2, &k1));

        let mut map: CtxHashMap<Key, u64> = CtxHashMap::new();
        assert_eq!(map.insert(k0, 42, &ctx), None);
        assert_eq!(map.insert(k2, 84, &ctx), Some(42));
        assert_eq!(map.get(&k1, &ctx), None);
        assert_eq!(*map.get(&k0, &ctx).unwrap(), 84);
    }

    #[test]
    fn test_entry() {
        let ctx = Ctx {
            vals: &["a", "b", "a"],
        };

        let k0 = Key { index: 0 };
        let k1 = Key { index: 1 };
        let k2 = Key { index: 2 };

        let mut map: CtxHashMap<Key, u64> = CtxHashMap::new();
        assert!(matches!(map.entry(k0), Entry::VacantEntry(_)));
    }
}
