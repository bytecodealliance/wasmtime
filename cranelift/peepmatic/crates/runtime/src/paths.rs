//! Representing paths through the dataflow graph.
//!
//! Paths are relative from a *root* instruction, which is the instruction we
//! are determining which, if any, optimizations apply.
//!
//! Paths are series of indices through each instruction's children as we
//! traverse down the graph from the root. Children are immediates followed by
//! arguments: `[imm0, imm1, ..., immN, arg0, arg1, ..., argN]`.
//!
//! ## Examples
//!
//! * `[0]` is the path to the root.
//! * `[0, 0]` is the path to the root's first child.
//! * `[0, 1]` is the path to the root's second child.
//! * `[0, 1, 0]` is the path to the root's second child's first child.
//!
//! ## Interning
//!
//! To avoid extra allocations, de-duplicate paths, and reference them via a
//! fixed-length value, we intern paths inside a `PathInterner` and then
//! reference them via `PathId`.

// TODO: Make `[]` the path to the root, and get rid of this redundant leading
// zero that is currently in every single path.

use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

/// A path through the data-flow graph from the root instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Path<'a>(pub &'a [u8]);

impl Path<'_> {
    /// Construct a new path through the data-flow graph from the root
    /// instruction.
    pub fn new(path: &impl AsRef<[u8]>) -> Path {
        Path(path.as_ref())
    }
}

/// An identifier for an interned path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathId(u16);

/// An interner and de-duplicator for `Path`s.
///
/// Can be serialized and deserialized while maintaining the same id to interned
/// path mapping.
#[derive(Debug, Default)]
pub struct PathInterner {
    /// A map from a path (whose owned data is inside `arena`) to the canonical
    /// `PathId` we assigned it when interning it.
    map: HashMap<UnsafePath, PathId>,

    /// A map from a `PathId` index to an unsafe, self-borrowed path pointing
    /// into `arena`. It is safe to given these out as safe `Path`s, as long as
    /// the lifetime is not longer than this `PathInterner`'s lifetime.
    paths: Vec<UnsafePath>,

    /// Bump allocation arena for path data. The bump arena ensures that these
    /// allocations never move, and are therefore safe for self-references.
    arena: Arc<bumpalo::Bump>,
}

impl PathInterner {
    /// Construct a new, empty `PathInterner`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    fn with_capacity(capacity: usize) -> Self {
        PathInterner {
            map: HashMap::with_capacity(capacity),
            paths: Vec::with_capacity(capacity),
            arena: Arc::new(bumpalo::Bump::new()),
        }
    }

    fn with_capacity_and_arena(capacity: usize, arena: Arc<bumpalo::Bump>) -> Self {
        PathInterner {
            map: HashMap::with_capacity(capacity),
            paths: Vec::with_capacity(capacity),
            arena,
        }
    }

    /// Intern a path into this `PathInterner`, returning its canonical
    /// `PathId`.
    ///
    /// If we've already interned this path before, then the existing id we
    /// already assigned to it is returned. If we've never seen this path
    /// before, then it is copied into this `PathInterner` and a new id is
    /// assigned to it.
    #[inline]
    pub fn intern<'a>(&mut self, path: Path<'a>) -> PathId {
        let unsafe_path = unsafe { UnsafePath::from_path(&path) };
        if let Some(id) = self.map.get(&unsafe_path) {
            return *id;
        }
        self.intern_new(path)
    }

    #[inline(never)]
    fn intern_new<'a>(&mut self, path: Path<'a>) -> PathId {
        let id = self.next_id();

        let our_path = self.arena.alloc_slice_copy(&path.0);
        let unsafe_path = unsafe { UnsafePath::from_slice(&our_path) };

        self.insert_path_with_id(id, unsafe_path.clone());

        debug_assert_eq!(self.lookup(id), path);
        debug_assert_eq!(self.intern(path), id);

        id
    }

    fn next_id(&self) -> PathId {
        let id: u16 = self
            .paths
            .len()
            .try_into()
            .expect("too many paths interned");
        PathId(id)
    }

    fn insert_path_with_id(&mut self, id: PathId, unsafe_path: UnsafePath) {
        self.paths.push(unsafe_path.clone());
        let old = self.map.insert(unsafe_path, id);

        debug_assert!(old.is_none());
    }

    /// Lookup a previously interned path by id.
    #[inline]
    pub fn lookup<'a>(&'a self, id: PathId) -> Path<'a> {
        let unsafe_path = self.lookup_unsafe_path(id);
        unsafe { unsafe_path.as_path() }
    }

    fn lookup_unsafe_path(&self, id: PathId) -> &UnsafePath {
        self.paths
            .get(id.0 as usize)
            .unwrap_or_else(|| Self::lookup_failure())
    }

    #[inline(never)]
    fn lookup_failure() -> ! {
        panic!(
            "no path for the given id; this can only happen when mixing `PathId`s with different \
             `PathInterner`s"
        )
    }

    /// Create a `PathInternerCompactor` that can be used to trim unused `Path`s from
    /// this `PathInterner`.
    pub fn compact(&self) -> PathInternerCompactor {
        PathInternerCompactor::new(self)
    }
}

/// A struct used to incrementally build up a new PathInterner
#[derive(Debug)]
pub struct PathInternerCompactor<'old> {
    old: &'old PathInterner,
    new: PathInterner,
}

impl<'old> PathInternerCompactor<'old> {
    fn new(old: &'old PathInterner) -> Self {
        let new = PathInterner::with_capacity_and_arena(old.paths.len(), Arc::clone(&old.arena));

        Self { old, new }
    }

    /// Convert a `PathId` from an old `PathInterner` to a new `PathId`.
    pub fn map(&mut self, old_id: PathId) -> PathId {
        // Check for an existing id, if not pull the `UnsafePath` data
        // from the old interner and correctly insert it into the new
        // interner. This should reuse the allocation of the
        // old `bumpalo::Bump`.

        let unsafe_path = self.old.lookup_unsafe_path(old_id);
        if let Some(new_id) = self.new.map.get(&unsafe_path) {
            return *new_id;
        }

        let new_id = self.new.next_id();

        self.new.insert_path_with_id(new_id, unsafe_path.clone());

        new_id
    }

    /// Consume the compactor and produce a new `PathInterner`.
    pub fn finish(self) -> PathInterner {
        log::debug!(
            "Creating new PathInterner that {} fewer interned values.",
            self.old.paths.len() - self.new.paths.len()
        );

        self.new
    }
}

impl Serialize for PathInterner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.paths.len()))?;
        for p in &self.paths {
            let p = unsafe { p.as_path() };
            seq.serialize_element(&p)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for PathInterner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(PathInternerVisitor {
            marker: PhantomData,
        })
    }
}

struct PathInternerVisitor {
    marker: PhantomData<fn() -> PathInterner>,
}

impl<'de> Visitor<'de> for PathInternerVisitor {
    type Value = PathInterner;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a `peepmatic_runtime::paths::PathInterner`")
    }

    fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: SeqAccess<'de>,
    {
        const DEFAULT_CAPACITY: usize = 16;
        let capacity = access.size_hint().unwrap_or(DEFAULT_CAPACITY);

        let mut interner = PathInterner::with_capacity(capacity);

        while let Some(path) = access.next_element::<Path>()? {
            interner.intern(path);
        }

        Ok(interner)
    }
}

/// An unsafe, unchecked borrow of a path. Not for use outside of
/// `PathInterner`!
#[derive(Clone, Debug)]
struct UnsafePath {
    ptr: *const u8,
    len: usize,
}

impl PartialEq for UnsafePath {
    fn eq(&self, rhs: &UnsafePath) -> bool {
        unsafe { self.as_slice() == rhs.as_slice() }
    }
}

impl Eq for UnsafePath {}

impl Hash for UnsafePath {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        unsafe { self.as_slice().hash(hasher) }
    }
}

/// Safety: callers must ensure that the constructed values won't have unsafe
/// usages of `PartialEq`, `Eq`, or `Hash`.
impl UnsafePath {
    unsafe fn from_path(p: &Path) -> Self {
        Self::from_slice(&p.0)
    }

    unsafe fn from_slice(s: &[u8]) -> Self {
        UnsafePath {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }
}

/// Safety: callers must ensure that `'a` does not outlive the lifetime of the
/// underlying data.
impl UnsafePath {
    unsafe fn as_slice<'a>(&self) -> &'a [u8] {
        std::slice::from_raw_parts(self.ptr, self.len)
    }

    unsafe fn as_path<'a>(&self) -> Path<'a> {
        Path(self.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};
    use std::convert::TryFrom;
    use std::iter::successors;

    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct OrderedPathInterner(PathInterner);

    impl PartialEq for OrderedPathInterner {
        fn eq(&self, other: &OrderedPathInterner) -> bool {
            let lhs_iter = self.0.paths.iter().map(|p| unsafe { p.as_path() });
            let rhs_iter = other.0.paths.iter().map(|p| unsafe { p.as_path() });

            lhs_iter.eq(rhs_iter)
        }
    }

    impl fmt::Debug for OrderedPathInterner {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.debug_struct("OrderedPathInterner")
                .field(
                    "paths",
                    &self
                        .0
                        .paths
                        .iter()
                        .map(|p| unsafe { p.as_path() })
                        .collect::<Vec<_>>(),
                )
                .finish()
        }
    }

    fn fib_iter(skip: usize, take: usize) -> impl Iterator<Item = u64> {
        successors(Some((0, 1)), |(a, b): &(u64, u64)| {
            a.checked_add(*b).map(|c| (*b, c))
        })
        .skip(skip)
        .take(take)
        .map(|(i, _)| i)
    }

    fn fill_interner(interner: &mut PathInterner, num_paths: usize) -> Vec<PathId> {
        let full_path: Vec<u8> = fib_iter(20, num_paths)
            .map(|i| u8::try_from(i % 256).unwrap())
            .collect();

        (1..=num_paths)
            .map(|path_len| {
                let path = &full_path[..path_len];

                interner.intern(Path(&path))
            })
            .collect()
    }

    #[test]
    fn test_compact_interner() {
        let mut original_interner = PathInterner::new();
        let path_ids = fill_interner(&mut original_interner, 10);

        let mut compactor = original_interner.compact();

        let new_path_ids = &[
            (path_ids[1], compactor.map(path_ids[1])),
            (path_ids[3], compactor.map(path_ids[3])),
            (path_ids[5], compactor.map(path_ids[5])),
            (path_ids[7], compactor.map(path_ids[7])),
            (path_ids[9], compactor.map(path_ids[9])),
        ];

        let new_interner = compactor.finish();

        // Check that the paths that were `map`ped are still present
        for (old_id, new_id) in new_path_ids {
            assert_eq!(
                original_interner.lookup(*old_id),
                new_interner.lookup(*new_id)
            );
        }

        // Check that the `PathId`s that were not `map`ped are not present
        for unused_path_id in &[
            path_ids[0],
            path_ids[2],
            path_ids[4],
            path_ids[6],
            path_ids[8],
        ] {
            let old_unsafe_path = original_interner.lookup_unsafe_path(*unused_path_id);

            // TODO: this test is rather intrusive to the internals of the PathInterner,
            // maybe there is a better way to do this.
            assert!(matches!(new_interner.map.get(old_unsafe_path), None));
        }
    }

    #[test]
    fn test_ser_de_empty_interner() {
        let interner = PathInterner::new();

        assert_tokens(
            &OrderedPathInterner(interner),
            &[Token::Seq { len: Some(0) }, Token::SeqEnd],
        );
    }

    #[test]
    fn test_ser_de_fib_path_interner() {
        let mut interner = PathInterner::new();
        fill_interner(&mut interner, 4);

        // NOTE: The serialized and deserialized forms are different
        // for somewhat unknown reasons.

        assert_ser_tokens(
            &interner,
            &[
                Token::Seq { len: Some(4) },
                // first path
                Token::NewtypeStruct { name: "Path" },
                Token::Seq { len: Some(1) },
                Token::U8(109),
                Token::SeqEnd,
                // second path
                Token::NewtypeStruct { name: "Path" },
                Token::Seq { len: Some(2) },
                Token::U8(109),
                Token::U8(194),
                Token::SeqEnd,
                // third path
                Token::NewtypeStruct { name: "Path" },
                Token::Seq { len: Some(3) },
                Token::U8(109),
                Token::U8(194),
                Token::U8(47),
                Token::SeqEnd,
                // first path
                Token::NewtypeStruct { name: "Path" },
                Token::Seq { len: Some(4) },
                Token::U8(109),
                Token::U8(194),
                Token::U8(47),
                Token::U8(241),
                Token::SeqEnd,
                // end
                Token::SeqEnd,
            ],
        );

        assert_de_tokens(
            &OrderedPathInterner(interner),
            &[
                Token::Seq { len: Some(4) },
                // first path
                Token::NewtypeStruct { name: "Path" },
                Token::BorrowedBytes(&[109]),
                // second path
                Token::NewtypeStruct { name: "Path" },
                Token::BorrowedBytes(&[109, 194]),
                // third path
                Token::NewtypeStruct { name: "Path" },
                Token::BorrowedBytes(&[109, 194, 47]),
                // first path
                Token::NewtypeStruct { name: "Path" },
                Token::BorrowedBytes(&[109, 194, 47, 241]),
                // end
                Token::SeqEnd,
            ],
        );
    }
}
